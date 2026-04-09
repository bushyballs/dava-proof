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
// Quote submission
// ---------------------------------------------------------------------------

/// Draft a quote-submission email.
///
/// * `sol`       – solicitation number (e.g. `W9127S26QA030`)
/// * `co_name`   – contracting officer full name
/// * `co_email`  – CO email address
/// * `attachments` – list of attachment file names to mention
pub fn quote_submit(
    sol: &str,
    co_name: &str,
    co_email: &str,
    attachments: &[&str],
) -> EmailDraft {
    let id = Identity::default();

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
pub fn amendment_ack(sol: &str, amendment_num: u32, co_email: &str) -> EmailDraft {
    let id = Identity::default();

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
pub fn debrief_request(sol: &str, co_name: &str, co_email: &str) -> EmailDraft {
    let id = Identity::default();

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
pub fn award_response(contract: &str, co_name: &str, co_email: &str) -> EmailDraft {
    let id = Identity::default();

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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_submit_subject_contains_sol() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[]);
        assert!(d.subject.contains("W9127S26QA030"));
    }

    #[test]
    fn quote_submit_to_is_co_email() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[]);
        assert_eq!(d.to, "ashley@usace.army.mil");
    }

    #[test]
    fn quote_submit_body_mentions_co_name() {
        let d = quote_submit("W9127S26QA030", "Ashley Stokes", "ashley@usace.army.mil", &[]);
        assert!(d.body.contains("Ashley Stokes"));
    }

    #[test]
    fn quote_submit_custom_attachments_listed() {
        let d = quote_submit(
            "TEST001",
            "Jane Doe",
            "jane@agency.gov",
            &["SF1449.pdf", "price_schedule.xlsx"],
        );
        assert!(d.body.contains("SF1449.pdf"));
        assert!(d.body.contains("price_schedule.xlsx"));
    }

    #[test]
    fn amendment_ack_subject_contains_amendment_number() {
        let d = amendment_ack("1240BE26Q0050", 1, "");
        assert!(d.subject.contains("Amendment 1"));
        assert!(d.subject.contains("1240BE26Q0050"));
    }

    #[test]
    fn amendment_ack_body_references_amendment() {
        let d = amendment_ack("1240BE26Q0050", 2, "co@agency.gov");
        assert!(d.body.contains("Amendment No. 2"));
        assert!(d.body.contains("1240BE26Q0050"));
    }

    #[test]
    fn debrief_request_cites_far_15506() {
        let d = debrief_request("127EAV26Q0031", "Ellena Silva", "ellena@agency.gov");
        assert!(d.body.contains("FAR 15.506"));
        assert!(d.subject.contains("FAR 15.506"));
    }

    #[test]
    fn debrief_request_to_is_co_email() {
        let d = debrief_request("127EAV26Q0031", "Ellena Silva", "ellena@agency.gov");
        assert_eq!(d.to, "ellena@agency.gov");
    }

    #[test]
    fn award_response_subject_contains_contract() {
        let d = award_response("12444626P0028", "John Smith", "john@agency.gov");
        assert!(d.subject.contains("12444626P0028"));
    }

    #[test]
    fn award_response_body_mentions_contract() {
        let d = award_response("12444626P0028", "", "");
        assert!(d.body.contains("12444626P0028"));
        // unnamed CO falls back to generic salutation
        assert!(d.body.contains("To Whom It May Concern"));
    }

    #[test]
    fn amendment_ack_zero_to_empty_string_when_no_email() {
        let d = amendment_ack("TESTSOL", 3, "");
        assert_eq!(d.to, "");
    }
}
