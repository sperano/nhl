use crate::data_provider::NHLDataProvider;
use anyhow::Result;

pub async fn run(client: &dyn NHLDataProvider) -> Result<()> {
    let franchises = client.franchises().await?;

    println!("\nNHL Franchises");
    println!("==============\n");

    println!(
        "{:<5} {:<40} {:<25} Place Name",
        "ID", "Full Name", "Common Name"
    );
    println!("{}", "─".repeat(100));

    for franchise in franchises {
        println!(
            "{:<5} {:<40} {:<25} {}",
            franchise.id,
            franchise.full_name,
            franchise.team_common_name,
            franchise.team_place_name
        );
    }

    println!();
    Ok(())
}
