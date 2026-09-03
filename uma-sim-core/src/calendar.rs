use crate::state::SimDate;

pub const CAREER_TURNS: i32 = 72;

#[derive(Debug, Clone)]
pub struct TurnCalendar {
    pub date: SimDate,
    pub turn: i32,
}

impl TurnCalendar {
    pub fn career_start() -> Self {
        Self {
            date: SimDate {
                year: 1,
                month: 6,
                half: 2,
            },
            turn: 1,
        }
    }

    pub fn advance(&self) -> Self {
        let next_turn = self.turn + 1;
        let mut month = self.date.month;
        let mut half = self.date.half;
        let mut year = self.date.year;
        if half == 1 {
            half = 2;
        } else {
            half = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
        Self {
            date: SimDate { year, month, half },
            turn: next_turn,
        }
    }

    pub fn label(&self) -> String {
        format!(
            "Year {} {} ({}), turn {}",
            self.date.year,
            month_name(self.date.month),
            if self.date.half == 1 { "Early" } else { "Late" },
            self.turn
        )
    }
}

fn month_name(m: i32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "M?",
    }
}
