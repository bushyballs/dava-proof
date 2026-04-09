/// Email template data returned by every template function.
#[derive(Debug, Clone)]
pub struct EmailDraft {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Company / signer constants pulled from env or hard-coded defaults.
pub struct Identity {
    pub company: String,
    pub signer: String,
    pub title: String,
    pub phone: String,
    pub email: String,
}

impl Default for Identity {
    fn default() -> Self {
        Identity {
            company: std::env::var("HOAGS_COMPANY")
                .unwrap_or_else(|_| "Hoags Inc.".to_string()),
            signer: std::env::var("HOAGS_SIGNER")
                .unwrap_or_else(|_| "Collin Hoag".to_string()),
            title: std::env::var("HOAGS_TITLE")
                .unwrap_or_else(|_| "Principal / Contracts Manager".to_string()),
            phone: std::env::var("HOAGS_PHONE")
                .unwrap_or_else(|_| "".to_string()),
            email: std::env::var("HOAGS_EMAIL")
                .unwrap_or_else(|_| "contracts@hoagsinc.com".to_string()),
        }
    }
}

impl Identity {
    /// Build an Identity from optional overrides, falling back to env/defaults.
    pub fn with_overrides(company: Option<&str>, signer: Option<&str>) -> Self {
        let mut id = Identity::default();
        if let Some(c) = company {
            if !c.is_empty() {
                id.company = c.to_string();
            }
        }
        if let Some(s) = signer {
            if !s.is_empty() {
                id.signer = s.to_string();
            }
        }
        id
    }
}

fn signature(id: &Identity) -> String {
    let phone_line = if id.phone.is_empty() {
        String::new()
    } else {
        format!("Phone:   {}\n", id.phone)
    };
    format!(
        "Respectfully,\n\n{}\n{}\n{}\n{}Email:   {}",
        id.signer, id.title, id.company, phone_line, id.email
    )
}

// ---------------------------------------------------------------------------
// Email validation
// ---------------------------------------------------------------------------

/// Validate that an email looks like a .gov or .mil address.
///
/// Returns `Ok(())` if valid, `Err(msg)` with a human-readable message if not.
/// An empty email string bypasses the check (some commands make CO email optional).
pub fn validate_gov_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Ok(());
    }
    let lower = email.to_lowercase();
    // Must contain @ with something on each side
    let at_pos = lower.find('@').ok_or_else(|| {
        format!("'{}' does not look like a valid email address (missing @)", email)
    })?;
    let domain = &lower[at_pos + 1..];
    if domain.ends_with(".gov") || domain.ends_with(".mil") {
        Ok(())
    } else {
        Err(format!(
            "'{}' does not appear to be a .gov or .mil address. \
             Federal CO emails must end in .gov or .mil.",
            email
        ))
    }
}

// ---------------------------------------------------------------------------
// Quote submission
// ---------------------------------------------------------------------------

/// Draft a quote-submission email.
///
/// * `sol`       – solicitation number (e.g. `W9127S26QA030`)
/// * `co_name`   – contracting officer full name
/// * `co_email`  – CO email address
/// * `attachments` – list of attachment file names to mention
/// * `company`   – override company name (None = use env/default)
/// * `signer`    – override signer name (None = use env/default)
pub fn quote_submit(
    sol: &str,
    co_name: &str,
    co_email: &str,
    attachments: &[&str],
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);

    let att_list = if attachments.is_empty() {
        "    • Completed SF 1449 / quote form\n    • Price schedule\n    • Any required representations and certifications".to_string()
    } else {
        attachments
            .iter()
            .map(|a| format!("    • {}", a))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let body = format!(
        r#"Dear {co_name},

Please find attached our firm's quote in response to Solicitation {sol}.
{company} is pleased to submit the following in accordance with the
requirements set forth in the solicitation documents.

Attached:
{att_list}

We have reviewed all solicitation documents, amendments (if any), and
performance work statements in full. Our pricing reflects the complete
scope of work and all applicable terms and conditions.

Should you require clarification or additional information, please do not
hesitate to contact us. We look forward to the opportunity to support your
mission.

{sig}"#,
        co_name = co_name,
        sol = sol,
        company = id.company,
        att_list = att_list,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!("Quote Submission — Solicitation {}", sol),
        body,
    }
}

// ---------------------------------------------------------------------------
// Amendment acknowledgment
// ---------------------------------------------------------------------------

/// Draft an amendment-acknowledgment email.
///
/// * `sol`            – solicitation number
/// * `amendment_num`  – amendment number (e.g. `1`)
/// * `co_email`       – CO email (optional; defaults to empty string)
/// * `company`        – override company name (None = use env/default)
/// * `signer`         – override signer name (None = use env/default)
pub fn amendment_ack(
    sol: &str,
    amendment_num: u32,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);

    let body = format!(
        r#"To Whom It May Concern,

This email serves as formal acknowledgment that {company} has received and
reviewed Amendment No. {amend} to Solicitation {sol}.

We confirm:
  • We have reviewed the full text of Amendment {amend}.
  • Our quote (if already submitted) will be revised as necessary to reflect
    any changes introduced by this amendment.
  • If our quote has not yet been submitted, this amendment has been
    incorporated into our preparation.

Please let us know if further acknowledgment or documentation is required.

{sig}"#,
        company = id.company,
        amend = amendment_num,
        sol = sol,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!(
            "Amendment {} Acknowledgment — Solicitation {}",
            amendment_num, sol
        ),
        body,
    }
}

// ---------------------------------------------------------------------------
// Debrief request (FAR 15.506)
// ---------------------------------------------------------------------------

/// Draft a post-award debrief-request email.
///
/// * `sol`     – solicitation number
/// * `co_name` – contracting officer full name
/// * `co_email`– CO email address
/// * `company` – override company name (None = use env/default)
/// * `signer`  – override signer name (None = use env/default)
pub fn debrief_request(
    sol: &str,
    co_name: &str,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);

    let body = format!(
        r#"Dear {co_name},

{company} respectfully requests a post-award debriefing pursuant to
FAR 15.506 for Solicitation {sol}.

We are requesting this debriefing to:
  • Understand the strengths and weaknesses of our submitted quote.
  • Obtain information that will help us improve future submissions.
  • Gain insight into the evaluation criteria and how our offer was assessed.

We understand the debriefing may be conducted in writing, by telephone, or
in person at the contracting office's discretion. We are flexible and can
accommodate any format convenient for your team.

Please advise us of the available format, date, and any information we
should prepare in advance. We appreciate the opportunity to improve our
competitiveness and look forward to working with your agency on future
procurements.

{sig}"#,
        co_name = co_name,
        company = id.company,
        sol = sol,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!(
            "Post-Award Debrief Request — Solicitation {} (FAR 15.506)",
            sol
        ),
        body,
    }
}

// ---------------------------------------------------------------------------
// Award response
// ---------------------------------------------------------------------------

/// Draft a response to an award notification.
///
/// * `contract`  – contract number (e.g. `12444626P0028`)
/// * `co_name`   – contracting officer full name (optional)
/// * `co_email`  – CO email address (optional)
/// * `company`   – override company name (None = use env/default)
/// * `signer`    – override signer name (None = use env/default)
pub fn award_response(
    contract: &str,
    co_name: &str,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);

    let salutation = if co_name.is_empty() {
        "To Whom It May Concern".to_string()
    } else {
        format!("Dear {}", co_name)
    };

    let body = format!(
        r#"{salutation},

{company} is pleased to acknowledge receipt of the award notification for
Contract {contract}. We are honored by the confidence placed in our firm and
are committed to delivering exceptional performance throughout the life of
this contract.

To ensure a smooth contract start-up, we would appreciate guidance on the
following:

  1. Point of contact for day-to-day contract administration (COR / COTR).
  2. Required reporting, invoicing, and documentation procedures.
  3. Performance start date and any mobilization or pre-performance
     requirements.
  4. Preferred communication channels and cadence for status updates.

We stand ready to begin performance and will comply fully with all terms,
conditions, and applicable regulations. Please do not hesitate to contact us
with any questions or requirements.

{sig}"#,
        salutation = salutation,
        company = id.company,
        contract = contract,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!("Award Acknowledgment — Contract {}", contract),
        body,
    }
}

// ---------------------------------------------------------------------------
// Status update (monthly)
// ---------------------------------------------------------------------------

/// Draft a monthly contract status-update email.
///
/// * `contract`  – contract number
/// * `status`    – free-text status string (e.g. "On track")
/// * `co_email`  – CO email address (optional)
/// * `company`   – override company name (None = use env/default)
/// * `signer`    – override signer name (None = use env/default)
pub fn status_update(
    contract: &str,
    status: &str,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);
    let month = chrono::Utc::now().format("%B %Y").to_string();

    let body = format!(
        r#"To Whom It May Concern,

{company} is pleased to provide the following monthly status update for
Contract {contract} — reporting period: {month}.

Current Status: {status}

Summary:
  • All performance objectives are being tracked and managed in accordance
    with the contract statement of work.
  • No significant issues, risks, or delays to report at this time unless
    noted in the status above.
  • Invoicing and documentation are current and in compliance with contract
    requirements.

Please do not hesitate to contact us if you require additional detail or
if any action items need to be addressed.

{sig}"#,
        company = id.company,
        contract = contract,
        month = month,
        status = status,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!("Monthly Status Update — Contract {} ({})", contract, month),
        body,
    }
}

// ---------------------------------------------------------------------------
// Question to CO
// ---------------------------------------------------------------------------

/// Draft a pre-solicitation question to the contracting officer.
///
/// * `sol`      – solicitation number
/// * `question` – the question text
/// * `co_email` – CO email address (optional)
/// * `company`  – override company name (None = use env/default)
/// * `signer`   – override signer name (None = use env/default)
pub fn question(
    sol: &str,
    question_text: &str,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);

    let body = format!(
        r#"To Whom It May Concern,

{company} respectfully submits the following question regarding
Solicitation {sol}:

  {question}

We appreciate your assistance in clarifying this matter. Should additional
information be needed to address our question, please do not hesitate to
contact us. We look forward to your response.

{sig}"#,
        company = id.company,
        sol = sol,
        question = question_text,
        sig = signature(&id),
    );

    EmailDraft {
        to: co_email.to_string(),
        subject: format!("Question Regarding Solicitation {}", sol),
        body,
    }
}

// ---------------------------------------------------------------------------
// Invoice submission
// ---------------------------------------------------------------------------

/// Draft an invoice submission email.
///
/// * `invoice_number` – invoice identifier (e.g. "HOAGS-INV-001")
/// * `amount`         – invoice amount in USD
/// * `contract`       – contract number (optional)
/// * `co_email`       – CO / finance office email (optional)
/// * `company`        – override company name (None = use env/default)
/// * `signer`         – override signer name (None = use env/default)
pub fn invoice_submit(
    invoice_number: &str,
    amount: f64,
    contract: Option<&str>,
    co_email: &str,
    company: Option<&str>,
    signer: Option<&str>,
) -> EmailDraft {
    let id = Identity::with_overrides(company, signer);
    let contract_line = contract
        .map(|c| format!("Contract Number: {}\n", c))
        .unwrap_or_default();
    let contract_ref = contract
        .map(|c| format!(" under Contract {}", c))
        .unwrap_or_default();

    let body = format!(
        r#"To Whom It May Concern,

{company} hereby submits the following invoice for services rendered{contract_ref}:

  Invoice Number:  {invoice_number}
  {contract_line}Invoice Amount:  ${amount:.2}

Please find the invoice attached to this correspondence. Payment should be
remitted in accordance with the terms set forth in the contract and applicable
prompt-payment regulations (31 U.S.C. § 3901 et seq.).

If you have any questions regarding this invoice or require additional
supporting documentation, please contact us at your earliest convenience.

{sig}"#,
        company = id.company,
        contract_ref = contract_ref,
        invoice_number = invoice_number,
        contract_line = contract_line,
        amount = amount,
        sig = signature(&id),
    );

    let subject = match contract {
        Some(c) => format!("Invoice Submission — {} — Contract {}", invoice_number, c),
        None => format!("Invoice Submission — {}", invoice_number),
    };

    EmailDraft {
        to: co_email.to_string(),
        subject,
        body,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- validate_gov_email ------------------------------------------------

    #[test]
    fn valid_gov_email_accepted() {
        assert!(validate_gov_email("co@usace.army.mil").is_ok());
        assert!(validate_gov_email("buyer@fs.usda.gov").is_ok());
    }

    #[test]
    fn non_gov_email_rejected() {
        let result = validate_gov_email("person@gmail.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".gov or .mil"));
    }

    #[test]
    fn empty_email_bypasses_validation() {
        assert!(validate_gov_email("").is_ok());
    }

    #[test]
    fn email_missing_at_sign_rejected() {
        assert!(validate_gov_email("notanemail").is_err());
    }

    // ---- Identity overrides ------------------------------------------------

    #[test]
    fn identity_overrides_company_and_signer() {
        let id = Identity::with_overrides(Some("ACME Corp"), Some("Jane Smith"));
        assert_eq!(id.company, "ACME Corp");
        assert_eq!(id.signer, "Jane Smith");
    }

    #[test]
    fn identity_empty_override_falls_back_to_default() {
        let id = Identity::with_overrides(Some(""), Some(""));
        // Falls back to env or default "Hoags Inc." / "Collin Hoag"
        assert!(!id.company.is_empty());
        assert!(!id.signer.is_empty());
    }

    // ---- quote_submit -------------------------------------------------------

    #[test]
    fn quote_submit_subject_contains_sol() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[], None, None);
        assert!(d.subject.contains("W9127S26QA030"));
    }

    #[test]
    fn quote_submit_to_is_co_email() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[], None, None);
        assert_eq!(d.to, "ashley@usace.army.mil");
    }

    #[test]
    fn quote_submit_body_mentions_co_name() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[], None, None);
        assert!(d.body.contains("Ashley Stokes"));
    }

    #[test]
    fn quote_submit_custom_attachments_listed() {
        let d = quote_submit(
            "TEST001",
            "Jane Doe",
            "jane@agency.gov",
            &["SF1449.pdf", "price_schedule.xlsx"],
            None,
            None,
        );
        assert!(d.body.contains("SF1449.pdf"));
        assert!(d.body.contains("price_schedule.xlsx"));
    }

    #[test]
    fn quote_submit_company_override() {
        let d = quote_submit("TEST001", "Jane Doe", "jane@agency.gov", &[], Some("ACME Corp"), Some("Alice"));
        assert!(d.body.contains("ACME Corp"));
        assert!(d.body.contains("Alice"));
    }

    // ---- amendment_ack -----------------------------------------------------

    #[test]
    fn amendment_ack_subject_contains_amendment_number() {
        let d = amendment_ack("1240BE26Q0050", 1, "", None, None);
        assert!(d.subject.contains("Amendment 1"));
        assert!(d.subject.contains("1240BE26Q0050"));
    }

    #[test]
    fn amendment_ack_body_references_amendment() {
        let d = amendment_ack("1240BE26Q0050", 2, "co@agency.gov", None, None);
        assert!(d.body.contains("Amendment No. 2"));
        assert!(d.body.contains("1240BE26Q0050"));
    }

    #[test]
    fn amendment_ack_zero_to_empty_string_when_no_email() {
        let d = amendment_ack("TESTSOL", 3, "", None, None);
        assert_eq!(d.to, "");
    }

    // ---- debrief_request ---------------------------------------------------

    #[test]
    fn debrief_request_cites_far_15506() {
        let d = debrief_request("127EAV26Q0031", "Ellena Silva", "ellena@agency.gov", None, None);
        assert!(d.body.contains("FAR 15.506"));
        assert!(d.subject.contains("FAR 15.506"));
    }

    #[test]
    fn debrief_request_to_is_co_email() {
        let d = debrief_request("127EAV26Q0031", "Ellena Silva", "ellena@agency.gov", None, None);
        assert_eq!(d.to, "ellena@agency.gov");
    }

    // ---- award_response ----------------------------------------------------

    #[test]
    fn award_response_subject_contains_contract() {
        let d = award_response("12444626P0028", "John Smith", "john@agency.gov", None, None);
        assert!(d.subject.contains("12444626P0028"));
    }

    #[test]
    fn award_response_body_mentions_contract() {
        let d = award_response("12444626P0028", "", "", None, None);
        assert!(d.body.contains("12444626P0028"));
        assert!(d.body.contains("To Whom It May Concern"));
    }

    // ---- status_update -----------------------------------------------------

    #[test]
    fn status_update_subject_contains_contract() {
        let d = status_update("12444626P0028", "On track", "co@usace.army.mil", None, None);
        assert!(d.subject.contains("12444626P0028"));
        assert!(d.subject.contains("Monthly Status Update"));
    }

    #[test]
    fn status_update_body_contains_status() {
        let d = status_update("12444626P0028", "On track", "co@usace.army.mil", None, None);
        assert!(d.body.contains("On track"));
    }

    #[test]
    fn status_update_body_contains_contract() {
        let d = status_update("12444626P0028", "On track", "", None, None);
        assert!(d.body.contains("12444626P0028"));
    }

    // ---- question ----------------------------------------------------------

    #[test]
    fn question_subject_contains_sol() {
        let d = question("W9127S26QA030", "What is the required security clearance?", "co@fs.usda.gov", None, None);
        assert!(d.subject.contains("W9127S26QA030"));
    }

    #[test]
    fn question_body_contains_question_text() {
        let d = question("W9127S26QA030", "What is the required security clearance?", "co@fs.usda.gov", None, None);
        assert!(d.body.contains("What is the required security clearance?"));
    }

    #[test]
    fn question_to_is_co_email() {
        let d = question("W9127S26QA030", "?", "co@fs.usda.gov", None, None);
        assert_eq!(d.to, "co@fs.usda.gov");
    }

    // ---- invoice_submit ----------------------------------------------------

    #[test]
    fn invoice_submit_subject_contains_invoice_number() {
        let d = invoice_submit("HOAGS-INV-001", 10645.63, Some("12444626P0028"), "finance@usace.army.mil", None, None);
        assert!(d.subject.contains("HOAGS-INV-001"));
    }

    #[test]
    fn invoice_submit_body_contains_amount() {
        let d = invoice_submit("HOAGS-INV-001", 10645.63, None, "", None, None);
        assert!(d.body.contains("10645.63"));
    }

    #[test]
    fn invoice_submit_body_contains_contract_when_provided() {
        let d = invoice_submit("HOAGS-INV-001", 500.0, Some("ABC123"), "", None, None);
        assert!(d.body.contains("ABC123"));
    }

    #[test]
    fn invoice_submit_subject_no_contract_when_absent() {
        let d = invoice_submit("HOAGS-INV-002", 100.0, None, "", None, None);
        assert!(d.subject.contains("HOAGS-INV-002"));
        assert!(!d.subject.contains("Contract"));
    }
}
