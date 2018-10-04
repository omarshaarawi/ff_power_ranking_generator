use std::cmp::Ordering;
use std::io;

extern crate requests;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
use requests::ToJson;

#[derive(Serialize, Deserialize)]
struct League {
    teams: Vec<Team>,

    #[serde(default = "default_usize")]
    league_size: usize,
}

impl League {
    fn get_max_points_for(&self) -> f64 {
        let mut max: f64 = 0.0;

        for team in self.teams.iter() {
            max = if team.record.points_for > max {
                team.record.points_for
            } else {
                max
            };
        }

        max
    }

    fn set_league_size(&mut self) {
        self.league_size = self.teams.len();
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Team {
    record: Record,
    team_id: u8,
    team_location: String,
    team_nickname: String,

    #[serde(default = "default_int")]
    overall_wins: u16,
}

fn default_int() -> u16 { 0 }
fn default_usize() -> usize { 0 }

impl Team {
    fn get_weeks_played(&self) -> u8 {
        self.record.overall_losses + self.record.overall_wins
    }

    fn calculate_win_percentage_weight(&self) -> f64 {
        let weeks_played = self.get_weeks_played();
        match weeks_played {
            1 => self.record.overall_percentage * 1.2,
            2 => self.record.overall_percentage * 2.4,
            _ => self.record.overall_percentage * 3.0,
        }
    }

    fn calculate_points_for_weight(&self, max_points_for: f64) -> f64 {
        self.record.points_for / max_points_for
    }

    fn calculate_overall_weight(&self, max_points_for: f64) -> f64 {
        let win_percentage = self.calculate_win_percentage_weight();
        let points_for_percentage = self.calculate_points_for_weight(max_points_for);
        win_percentage + points_for_percentage
    }

    fn compare(&self, team: &Team, max_points_for: f64) -> Ordering {
        let self_weight = self.calculate_overall_weight(max_points_for);
        let other_weight = team.calculate_overall_weight(max_points_for);

        other_weight.partial_cmp(&self_weight).unwrap_or(Ordering::Less)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    overall_losses: u8,
    overall_percentage: f64,
    overall_wins: u8,
    points_for: f64,
}

static BASE_URL: &'static str = "https://games.espn.com/ffl/api/v2/";

fn retrieve_league_id() -> u64 {
    let mut league_id = String::new();

    println!("Please enter your league ID.");

    io::stdin().read_line(&mut league_id)
        .expect("Failed to read line.");

    league_id.trim().parse().unwrap()
}

fn retrieve_league_data(league_id: u64) -> String {
    let request_url = format!("{0}teams?leagueId={1}", BASE_URL, league_id);

    requests::get(request_url).unwrap().json().unwrap().to_string()
}

fn print_results(league: League) {
    for (index, team) in league.teams.iter().enumerate() {
        println!("{}. {} {}", (index + 1), &team.team_location, &team.team_nickname);
    }
}

// Retrieve leagueSchedules
// Loop through each week
// Form tuple of (teamID, score)
// Sort by score

fn main() {
    let league_id = retrieve_league_id();

    let data = retrieve_league_data(league_id);

    let mut league: League = serde_json::from_str(&data).unwrap();
    league.set_league_size();

    let max_points_for = league.get_max_points_for();

    league.teams.sort_by(|a, b| a.compare(b, max_points_for));

    print_results(league);
}
