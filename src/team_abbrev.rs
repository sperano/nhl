//TODO this should go in nhl_api
// everywhere in the api where there a team info without an abbreviation, we should add a team_abbrev field and make it automatically set with this lookup table

/// Map team abbreviation to team common name
///
/// This function maps NHL team abbreviations (e.g., "TOR")
/// to their common names (e.g., "Maple Leafs").
pub fn abbrev_to_common_name(abbrev: &str) -> Option<&'static str> {
    match abbrev {
        "ANA" => Some("Ducks"),
        "ARI" => Some("Coyotes"),
        "BOS" => Some("Bruins"),
        "BUF" => Some("Sabres"),
        "CGY" => Some("Flames"),
        "CAR" => Some("Hurricanes"),
        "CHI" => Some("Blackhawks"),
        "COL" => Some("Avalanche"),
        "CBJ" => Some("Blue Jackets"),
        "DAL" => Some("Stars"),
        "DET" => Some("Red Wings"),
        "EDM" => Some("Oilers"),
        "FLA" => Some("Panthers"),
        "LAK" => Some("Kings"),
        "MIN" => Some("Wild"),
        "MTL" => Some("Canadiens"),
        "NSH" => Some("Predators"),
        "NJD" => Some("Devils"),
        "NYI" => Some("Islanders"),
        "NYR" => Some("Rangers"),
        "OTT" => Some("Senators"),
        "PHI" => Some("Flyers"),
        "PIT" => Some("Penguins"),
        "SJS" => Some("Sharks"),
        "SEA" => Some("Kraken"),
        "STL" => Some("Blues"),
        "TBL" => Some("Lightning"),
        "TOR" => Some("Maple Leafs"),
        "VAN" => Some("Canucks"),
        "VGK" => Some("Golden Knights"),
        "WSH" => Some("Capitals"),
        "WPG" => Some("Jets"),
        "UTA" => Some("Hockey Club"),
        // Historical teams
        "PHX" => Some("Phoenix Coyotes"),
        "ATL" => Some("Atlanta Thrashers"),
        _ => None,
    }
}

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
