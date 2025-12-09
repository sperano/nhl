//TODO this should go in nhl_api
// everywhere in the api where there a team info without an abbreviation, we should add a team_abbrev field and make it automatically set with this lookup table

/// Map team common name to team abbreviation
///
/// This function maps NHL team common names (e.g., "Maple Leafs")
/// to their standard 3-letter abbreviations (e.g., "TOR").
pub fn common_name_to_abbrev(common_name: &str) -> Option<&'static str> {
    match common_name {
        "Ducks" => Some("ANA"),
        "Coyotes" => Some("ARI"),
        "Bruins" => Some("BOS"),
        "Sabres" => Some("BUF"),
        "Flames" => Some("CGY"),
        "Hurricanes" => Some("CAR"),
        "Blackhawks" => Some("CHI"),
        "Avalanche" => Some("COL"),
        "Blue Jackets" => Some("CBJ"),
        "Stars" => Some("DAL"),
        "Red Wings" => Some("DET"),
        "Oilers" => Some("EDM"),
        "Panthers" => Some("FLA"),
        "Kings" => Some("LAK"),
        "Wild" => Some("MIN"),
        "Canadiens" => Some("MTL"),
        "Predators" => Some("NSH"),
        "Devils" => Some("NJD"),
        "Islanders" => Some("NYI"),
        "Rangers" => Some("NYR"),
        "Senators" => Some("OTT"),
        "Flyers" => Some("PHI"),
        "Penguins" => Some("PIT"),
        "Sharks" => Some("SJS"),
        "Kraken" => Some("SEA"),
        "Blues" => Some("STL"),
        "Lightning" => Some("TBL"),
        "Maple Leafs" => Some("TOR"),
        "Canucks" => Some("VAN"),
        "Golden Knights" => Some("VGK"),
        "Capitals" => Some("WSH"),
        "Jets" => Some("WPG"),
        "Hockey Club" => Some("UTA"),
        // Historical teams
        "Phoenix Coyotes" => Some("PHX"),
        "Atlanta Thrashers" => Some("ATL"),
        _ => None,
    }
}
