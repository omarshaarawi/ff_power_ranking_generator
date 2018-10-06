use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::iter::Sum;

extern crate requests;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
use requests::ToJson;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeagueSchedule {
    league_schedule: Schedule,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Schedule {
    schedule_items: Vec<Week>,
}

impl Schedule {
    fn calculate_overall_wins(&self) -> HashMap<u8, u16> {
        let mut score_map: HashMap<u8, u16> = HashMap::new();
        for week in self.schedule_items.iter() {
            let mut week_scores = week.get_week_scores();
            week_scores.sort_by(|a, b| a.compare(b));
            for (index, score) in week_scores.iter().enumerate() {
                let num_wins = week_scores.len() - (index + 1);
                let num_wins = num_wins as u16;
                let total_wins = score_map.entry(score.team_id).or_insert(0);
                *total_wins += num_wins;
            }
        }
        score_map
    }
}

#[derive(Serialize, Deserialize)]
struct Week {
    matchups: Vec<Matchup>,
}

impl Week {
    fn get_week_scores(&self) -> Vec<Score> {
        let mut scores: Vec<Score> = Vec::new();
        for matchup in self.matchups.iter() {
            if matchup.outcome != 0 {
                scores.append(&mut matchup.get_matchup_scores());
            }
        }
        scores
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Matchup {
    away_team_id: u8,
    home_team_id: u8,
    away_team_scores: Vec<f64>,
    home_team_scores: Vec<f64>,
    outcome: u8,
}

impl Matchup {
    fn get_matchup_scores(&self) -> Vec<Score> {
        vec![
            Score{team_id: self.away_team_id, score: self.away_team_scores[0]},
            Score{team_id: self.home_team_id, score: self.home_team_scores[0]},
        ]
    }
}

#[derive(Debug)]
struct Score {
    team_id: u8,
    score: f64,
}

impl Score {
    fn compare(&self, other: &Score) -> Ordering {
        other.score.partial_cmp(&self.score).unwrap_or(Ordering::Less)
    }
}

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

    fn calculate_total_wins_weight(&self, total_wins: u16) -> f64 {
        let possible_total_wins = self.calculate_possible_total_wins();
        let possible_total_wins = possible_total_wins as f64;
        let tot_wins = total_wins as f64;
        tot_wins / possible_total_wins
    }

    fn calculate_possible_total_wins(&self) -> u16 {
        let weeks_played = self.get_weeks_played() as u16;
        let teams: u16 = 10;
        let team_sum: u16 = (0..teams+1).fold(0, |a, b| a + b);
        (team_sum * weeks_played) - (teams * weeks_played)
    }

    fn calculate_overall_weight(&self, max_points_for: f64, total_wins: u16) -> f64 {
        let win_percentage = self.calculate_win_percentage_weight();
        let points_for_percentage = self.calculate_points_for_weight(max_points_for);
        let total_wins_percentage = self.calculate_total_wins_weight(total_wins);
        win_percentage + points_for_percentage + total_wins_percentage
    }

    fn compare(&self, team: &Team, max_points_for: f64, total_wins_map: &HashMap<u8, u16>) -> Ordering {
        let self_total_wins = match total_wins_map.get(&self.team_id) {
            Some(i) => *i,
            None => 0,
        };
        let team_total_wins = match total_wins_map.get(&team.team_id) {
            Some(i) => *i,
            None => 0,
        };

        let self_weight = self.calculate_overall_weight(max_points_for, self_total_wins);
        let other_weight = team.calculate_overall_weight(max_points_for, team_total_wins);

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

fn make_request(request_url: String) -> String {
    requests::get(request_url).unwrap().json().unwrap().to_string()
}

fn retrieve_league_data(league_id: u64) -> String {
    let request_url = format!("{0}teams?leagueId={1}", BASE_URL, league_id);
    make_request(request_url)
}

fn retrieve_league_schedule(league_id: u64) -> String {
    let request_url = format!("{0}leagueSchedules?leagueId={1}", BASE_URL, league_id);
    make_request(request_url)
}

fn print_results(league: League) {
    for (index, team) in league.teams.iter().enumerate() {
        println!("{}. {} {}", (index + 1), &team.team_location, &team.team_nickname);
    }
}

fn main() {
    let league_id = retrieve_league_id();

    let league_data = retrieve_league_data(league_id);
    let mut league: League = serde_json::from_str(&league_data).unwrap();
    league.set_league_size();

    let league_schedule = retrieve_league_schedule(league_id);
    let league_schedule: LeagueSchedule = serde_json::from_str(&league_schedule).unwrap();

    let total_wins_map = league_schedule.league_schedule.calculate_overall_wins();

    let max_points_for = league.get_max_points_for();

    league.teams.sort_by(|a, b| a.compare(b, max_points_for, &total_wins_map));

    print_results(league);
}
