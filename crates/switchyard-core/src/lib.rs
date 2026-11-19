use serde::{Deserialize, Serialize};

pub const COLS: usize = 7;
pub const ROWS: usize = 6;
const SQUARES: usize = COLS * ROWS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Point {
    fn index(self) -> usize {
        self.y * COLS + self.x
    }

    pub fn step(self, heading: Direction) -> Option<Self> {
        let (dx, dy) = match heading {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        };
        let x = self.x.checked_add_signed(dx)?;
        let y = self.y.checked_add_signed(dy)?;
        (x < COLS && y < ROWS).then_some(Self { x, y })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn opposite(self) -> Self {
        use Direction::*;
        match self {
            North => South,
            East => West,
            South => North,
            West => East,
        }
    }

    fn bit(self) -> u8 {
        1 << self as u8
    }

    pub fn branches(self) -> [Self; 2] {
        use Direction::*;
        match self {
            North | South => [East, West],
            East | West => [North, South],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Rail(Direction, Direction),
    Junction(Direction),
    Start(Direction),
    Depot,
    Buffer,
}

#[derive(Clone, Copy)]
pub struct Yard {
    pub title: &'static str,
    rows: &'static [&'static str; ROWS],
    initial: u16,
}

impl Yard {
    pub fn tile(&self, point: Point) -> Tile {
        use Direction::*;
        use Tile::*;
        match self.rows[point.y].as_bytes()[point.x] {
            b'-' => Rail(West, East),
            b'|' => Rail(North, South),
            b'L' => Rail(North, East),
            b'J' => Rail(North, West),
            b'r' => Rail(South, East),
            b'7' => Rail(South, West),
            b'U' => Junction(North),
            b'E' => Junction(East),
            b'D' => Junction(South),
            b'W' => Junction(West),
            b'>' => Start(East),
            b'<' => Start(West),
            b'v' => Start(South),
            b'G' => Depot,
            b'x' => Buffer,
            _ => Empty,
        }
    }

    pub fn points() -> impl Iterator<Item = Point> {
        (0..ROWS).flat_map(|y| (0..COLS).map(move |x| Point { x, y }))
    }

    fn junctions(&self) -> impl Iterator<Item = Point> + '_ {
        Self::points().filter(|&point| matches!(self.tile(point), Tile::Junction(_)))
    }

    fn junction_index(&self, at: Point) -> Option<usize> {
        self.junctions().position(|point| point == at)
    }

    pub fn junction_count(&self) -> usize {
        self.junctions().count()
    }

    fn start(&self) -> (Point, Direction) {
        Self::points()
            .find_map(|point| match self.tile(point) {
                Tile::Start(heading) => Some((point, heading)),
                _ => None,
            })
            .expect("every yard includes a train")
    }
}

pub const YARDS: &[Yard] = &[
    Yard {
        title: "Depot lane",
        rows: &[
            "       ", "       ", "  r--G ", "  |    ", ">-W    ", "  x    ",
        ],
        initial: 1,
    },
    Yard {
        title: "North siding",
        rows: &[
            "       ", " xD-7  ", "  | |  ", ">-W |  ", "  x L-G", "       ",
        ],
        initial: 1,
    },
    Yard {
        title: "Across town",
        rows: &[
            " v     ", " | x G ", "xU-W | ", "   | | ", "   L-J ", "       ",
        ],
        initial: 0,
    },
    Yard {
        title: "West freight",
        rows: &[
            "       ", "  r-Dx ", "  | |  ", "  | E-<", "G-J x  ", "       ",
        ],
        initial: 0,
    },
    Yard {
        title: "Coal bend",
        rows: &[
            "       ", " x r-7 ", ">W | | ", " | | LG", "xU-W   ", "   x   ",
        ],
        initial: 0,
    },
    Yard {
        title: "Evening turn",
        rows: &[
            "       ", "G7 x   ", " | E-Dx", " | | | ", " L-UxE<", "     x ",
        ],
        initial: 0,
    },
    Yard {
        title: "Signal hill",
        rows: &[
            "   x   ", "xD-W rG", " | | | ", " |xU-J ", ">W     ", " x     ",
        ],
        initial: 0,
    },
    Yard {
        title: "Boxcar run",
        rows: &[
            "       ", "  x r-G", "xDW |  ", " || |  ", ">WL-W  ", " x  x  ",
        ],
        initial: 0,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Delivered,
    Buffer,
    Blocked,
    Loop,
}

#[derive(Clone, Debug)]
pub struct Route {
    pub points: Vec<Point>,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    current: usize,
    settings: Vec<u16>,
    completed: Vec<bool>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            current: 0,
            settings: vec![0; YARDS.len()],
            completed: vec![false; YARDS.len()],
        }
    }
}

impl Game {
    pub fn restored(mut self) -> Self {
        self.current = self.current.min(YARDS.len() - 1);
        self.settings.resize_with(YARDS.len(), u16::default);
        self.completed.resize(YARDS.len(), false);
        self
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn yard(&self) -> Yard {
        YARDS[self.current]
    }

    pub fn completed(&self) -> usize {
        self.completed.iter().filter(|&&done| done).count()
    }

    pub fn go(&mut self, index: usize) -> bool {
        if index >= YARDS.len() || index == self.current {
            return false;
        }
        self.current = index;
        true
    }

    pub fn reset(&mut self) {
        self.settings[self.current] = 0;
    }

    pub fn toggle(&mut self, point: Point) -> bool {
        let Some(index) = self.yard().junction_index(point) else {
            return false;
        };
        self.settings[self.current] ^= 1 << index;
        true
    }

    pub fn branch(&self, point: Point, common: Direction) -> Direction {
        let yard = self.yard();
        let index = yard
            .junction_index(point)
            .expect("a branch belongs to a junction");
        let flipped = (self.settings[self.current] ^ yard.initial) >> index & 1;
        common.branches()[flipped as usize]
    }

    pub fn dispatch(&mut self) -> Route {
        let route = self.route();
        if route.outcome == Outcome::Delivered {
            self.completed[self.current] = true;
        }
        route
    }

    fn route(&self) -> Route {
        let yard = self.yard();
        let (start, mut heading) = yard.start();
        let mut point = start;
        let mut points = vec![point];
        let mut visited = [0; SQUARES];
        let outcome = loop {
            let Some(next) = point.step(heading) else {
                break Outcome::Blocked;
            };
            let entry = heading.opposite();
            let recent = &mut visited[next.index()];
            if *recent & entry.bit() != 0 {
                break Outcome::Loop;
            }
            *recent |= entry.bit();
            let (a, b) = match yard.tile(next) {
                Tile::Depot => {
                    points.push(next);
                    break Outcome::Delivered;
                }
                Tile::Buffer => {
                    points.push(next);
                    break Outcome::Buffer;
                }
                Tile::Rail(a, b) => (a, b),
                Tile::Junction(common) => (common, self.branch(next, common)),
                Tile::Empty | Tile::Start(_) => break Outcome::Blocked,
            };
            heading = if entry == a {
                b
            } else if entry == b {
                a
            } else {
                break Outcome::Blocked;
            };
            points.push(next);
            point = next;
        };
        Route { points, outcome }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_yard_begins_unsolved_and_has_exactly_one_working_switch_setting() {
        let mut game = Game::default().restored();
        for (current, yard) in YARDS.iter().enumerate() {
            assert!(yard.rows.iter().all(|row| row.len() == COLS));
            game.current = current;
            assert_ne!(game.route().outcome, Outcome::Delivered, "{}", yard.title);
            let solutions = (0..(1 << yard.junction_count()))
                .filter(|&setting| {
                    game.settings[current] = setting;
                    game.route().outcome == Outcome::Delivered
                })
                .count();
            assert_eq!(solutions, 1, "{}", yard.title);
        }
    }

    #[test]
    fn dispatch_records_the_first_yard_only_after_the_switch_changes() {
        let mut game = Game::default().restored();
        assert_eq!(game.dispatch().outcome, Outcome::Buffer);
        assert_eq!(game.completed(), 0);
        assert!(game.toggle(Point { x: 2, y: 4 }));
        let route = game.dispatch();
        assert_eq!(route.outcome, Outcome::Delivered);
        assert_eq!(route.points.last(), Some(&Point { x: 5, y: 2 }));
        assert_eq!(game.completed(), 1);
        game.reset();
        assert_eq!(game.dispatch().outcome, Outcome::Buffer);
        assert_eq!(game.completed(), 1);
    }
}
