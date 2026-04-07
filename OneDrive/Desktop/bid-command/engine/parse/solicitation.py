"""engine.parse.solicitation — Extract structured fields from raw solicitation text."""

from __future__ import annotations

import re
from dataclasses import dataclass, field


@dataclass
class SolicitationData:
    sol_number: str = ""
    title: str = ""
    naics: str = ""
    set_aside: str = ""
    due_date: str = ""
    co_name: str = ""
    co_email: str = ""
    co_phone: str = ""
    agency: str = ""
    state: str = ""
    city: str = ""
    zip_code: str = ""
    delivery_address: str = ""
    psc_code: str = ""
    submission_email: str = ""


# ---------------------------------------------------------------------------
# Compiled patterns
# ---------------------------------------------------------------------------

# Solicitation numbers: start with digits or letters, mix of alpha+digit, 10-20 chars
# Examples: 1240BF26Q0027, W912DQ26QA045, SPE7LX26T0001
_SOL_NUMBER_RE = re.compile(
    r"\b([A-Z0-9]{2,6}[A-Z]{1,3}[0-9]{2}[A-Z]{0,2}[0-9]{3,6})\b"
)

# 6-digit NAICS codes
_NAICS_RE = re.compile(r"\b([1-9]\d{5})\b")

# Due date: MM/DD/YYYY optionally followed by time + timezone
_DUE_DATE_RE = re.compile(
    r"\b(\d{2}/\d{2}/\d{4}(?:\s+\d{3,4}(?:\s+[A-Z]{2,3})?)?)\b"
)

# Phone: (XXX) XXX-XXXX or XXX-XXX-XXXX
_PHONE_RE = re.compile(r"(\(?\d{3}\)?[\s.\-]\d{3}[\-.\s]\d{4})")

# Any @*.gov email
_GOV_EMAIL_RE = re.compile(r"[\w.\-+]+@[\w.\-]+\.gov", re.IGNORECASE)

# Submission email — email appearing after "submit" or "email to" (case-insensitive)
_SUBMIT_CTX_RE = re.compile(
    r"(?:submit|email\s+to|submitted\s+(?:via|to)\s+email\s+to)\b[^@\n]{0,60}?([\w.\-+]+@[\w.\-]+\.gov)",
    re.IGNORECASE,
)

# Set-aside keywords (order matters — more specific first)
_SET_ASIDE_PATTERNS: list[tuple[re.Pattern, str]] = [
    (re.compile(r"\bSDBVOSB\b", re.IGNORECASE), "SDBVOSB"),
    (re.compile(r"\bSDVOSB\b", re.IGNORECASE), "SDVOSB"),
    (re.compile(r"\bHUBZONE\b", re.IGNORECASE), "HUBZONE"),
    (re.compile(r"\b8\s*\(A\)\b", re.IGNORECASE), "8(A)"),
    (re.compile(r"\bWOSB\b", re.IGNORECASE), "WOSB"),
    (re.compile(r"\bEDWOSB\b", re.IGNORECASE), "EDWOSB"),
    (re.compile(r"SMALL\s+BUSINESS", re.IGNORECASE), "SMALL BUSINESS"),
    (re.compile(r"TOTAL\s+SMALL\s+BUSINESS", re.IGNORECASE), "TOTAL SMALL BUSINESS"),
]

# City/State/ZIP: UPPERCASE WORD(S) followed by 2-letter state abbreviation and 5-digit ZIP
_LOCATION_RE = re.compile(
    r"\b([A-Z][A-Z ]{1,24}?)\s+([A-Z]{2})\s+(\d{5}(?:-\d{4})?)\b"
)

# PSC / Product-Service Code: "Product/Service Code:" then 1-6 alphanumeric chars
_PSC_RE = re.compile(r"Product/Service\s+Code[:\s]+([A-Z0-9]{1,6})", re.IGNORECASE)

# US state abbreviations for validation
_US_STATES = {
    "AL","AK","AZ","AR","CA","CO","CT","DE","FL","GA","HI","ID","IL","IN","IA",
    "KS","KY","LA","ME","MD","MA","MI","MN","MS","MO","MT","NE","NV","NH","NJ",
    "NM","NY","NC","ND","OH","OK","OR","PA","RI","SC","SD","TN","TX","UT","VT",
    "VA","WA","WV","WI","WY","DC","GU","PR","VI","AS","MP",
}


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------

def parse_solicitation(text: str) -> SolicitationData:
    """Extract structured fields from raw solicitation text using regex heuristics."""
    data = SolicitationData()

    # --- Sol number ---
    for m in _SOL_NUMBER_RE.finditer(text):
        candidate = m.group(1)
        # Must contain both letters and digits, at least 8 chars
        if re.search(r"[A-Z]", candidate) and re.search(r"\d", candidate) and len(candidate) >= 8:
            data.sol_number = candidate
            break

    # --- NAICS (6-digit) ---
    # Prefer codes commonly associated with services; take first valid 6-digit hit
    for m in _NAICS_RE.finditer(text):
        code = m.group(1)
        data.naics = code
        break

    # --- Due date ---
    m = _DUE_DATE_RE.search(text)
    if m:
        data.due_date = m.group(1).strip()

    # --- Emails ---
    all_gov_emails = _GOV_EMAIL_RE.findall(text)

    # Submission email — context-sensitive search first
    sub_m = _SUBMIT_CTX_RE.search(text)
    if sub_m:
        data.submission_email = sub_m.group(1).lower()
    elif all_gov_emails:
        data.submission_email = all_gov_emails[-1].lower()

    # CO email — first gov email found in document
    if all_gov_emails:
        data.co_email = all_gov_emails[0].lower()

    # --- Phone ---
    m = _PHONE_RE.search(text)
    if m:
        data.co_phone = m.group(1).strip()

    # --- CO name (name appearing before a phone number) ---
    phone_m = _PHONE_RE.search(text)
    if phone_m:
        # Look at the 3 lines immediately before the phone number
        before = text[: phone_m.start()]
        lines_before = [ln.strip() for ln in before.splitlines() if ln.strip()]
        if lines_before:
            candidate_name = lines_before[-1]
            # Accept if it looks like a name: 1-4 words, only letters/spaces/hyphens/apostrophes
            if re.match(r"^[A-Za-z][A-Za-z' \-]{1,50}$", candidate_name) and len(candidate_name.split()) <= 4:
                data.co_name = candidate_name

    # --- Set-aside ---
    for pattern, label in _SET_ASIDE_PATTERNS:
        if pattern.search(text):
            data.set_aside = label
            break

    # --- Location (city / state / ZIP) ---
    for m in _LOCATION_RE.finditer(text):
        city_candidate = m.group(1).strip()
        state_candidate = m.group(2)
        zip_candidate = m.group(3)
        if state_candidate in _US_STATES:
            data.city = city_candidate
            data.state = state_candidate
            data.zip_code = zip_candidate
            break

    # --- PSC code ---
    m = _PSC_RE.search(text)
    if m:
        data.psc_code = m.group(1).upper()

    return data
