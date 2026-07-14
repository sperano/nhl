use crate::data_provider::NHLDataProvider;
use anyhow::Result;
use nhl_api::Franchise;

/// Width of the separator line under the column headers.
const SEPARATOR_WIDTH: usize = 100;

pub async fn run(client: &dyn NHLDataProvider) -> Result<()> {
    let franchises = client.franchises().await?;
    print!("{}", format_franchises(&franchises));

    Ok(())
}

/// Build the franchise listing (title, column headers, and one row per franchise) as a
/// string, so the formatting logic can be unit-tested without capturing stdout.
fn format_franchises(franchises: &[Franchise]) -> String {
    let mut output = String::new();

    output.push_str("\nNHL Franchises\n");
    output.push_str("==============\n\n");

    output.push_str(&format!(
        "{:<5} {:<40} {:<25} Place Name\n",
        "ID", "Full Name", "Common Name"
    ));
    output.push_str(&"─".repeat(SEPARATOR_WIDTH));
    output.push('\n');

    for franchise in franchises {
        output.push_str(&format!(
            "{:<5} {:<40} {:<25} {}\n",
            franchise.id,
            franchise.full_name,
            franchise.team_common_name,
            franchise.team_place_name
        ));
    }

    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::mock_client::MockClient;
    use crate::fixtures;

    #[test]
    fn test_format_franchises_includes_title_and_column_headers() {
        let output = format_franchises(&fixtures::create_mock_franchises());

        assert!(output.contains("NHL Franchises"));
        assert!(output.contains("ID"));
        assert!(output.contains("Full Name"));
        assert!(output.contains("Common Name"));
        assert!(output.contains("Place Name"));
        assert!(output.contains(&"─".repeat(SEPARATOR_WIDTH)));
    }

    #[test]
    fn test_format_franchises_includes_a_row_per_franchise() {
        let franchises = fixtures::create_mock_franchises();
        let output = format_franchises(&franchises);

        for franchise in &franchises {
            assert!(output.contains(&franchise.full_name));
            assert!(output.contains(&franchise.team_common_name));
            assert!(output.contains(&franchise.team_place_name));
        }
        // One row per franchise, plus 7 fixed lines: leading blank, title, "====" rule,
        // blank, column header, separator rule, and trailing blank.
        const FIXED_LINE_COUNT: usize = 7;
        assert_eq!(
            output.lines().count(),
            franchises.len() + FIXED_LINE_COUNT,
            "expected one line per franchise plus the fixed header/footer lines"
        );
    }

    #[test]
    fn test_format_franchises_empty_list_renders_headers_only() {
        let output = format_franchises(&[]);

        assert!(output.contains("NHL Franchises"));
        assert_eq!(output.lines().count(), 7);
    }

    #[test]
    fn test_format_franchises_handles_unicode_names_without_panicking() {
        let franchises = vec![Franchise {
            id: 99,
            full_name: "Montréal Canadiens Ünïcode".to_string(),
            team_common_name: "Canadiens".to_string(),
            team_place_name: "Montréal".to_string(),
        }];

        let output = format_franchises(&franchises);

        assert!(output.contains("Montréal Canadiens Ünïcode"));
        assert!(output.contains("Montréal"));
    }

    #[tokio::test]
    async fn test_run_returns_ok() {
        let client = MockClient::new();
        let result = run(&client).await;
        assert!(result.is_ok());
    }
}
