use std::{fs, io, path::PathBuf};

use primitive_app::{App, Change};
use primitive_ui::{
    back_button, header, screen_rect, Canvas, Input, Rect, TextStyle, BLACK, BODY, CAPTION,
    HAIRLINE, PAPER, SECONDARY,
};
use switchyard_core::{Direction, Game, Outcome, Point, Route, Tile, Yard, COLS, ROWS, YARDS};

const LEFT: i32 = 72;
const TOP: i32 = 300;
const UNIT: i32 = 160;

pub struct View {
    pub game: Game,
    path: Option<PathBuf>,
    route: Option<Route>,
    save_failed: bool,
}

impl View {
    pub fn open(path: PathBuf) -> Self {
        let game = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Game>(&bytes).ok())
            .unwrap_or_default()
            .restored();
        Self {
            game,
            path: Some(path),
            route: None,
            save_failed: false,
        }
    }

    pub fn preview() -> Self {
        Self {
            game: Game::default(),
            path: None,
            route: None,
            save_failed: false,
        }
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.route.as_ref().map(|route| route.outcome)
    }

    fn persist(&mut self) {
        self.save_failed = self.save().is_err();
    }

    fn page(&mut self, forward: bool) -> Change {
        let index = self.game.current();
        let next = if forward {
            index + 1
        } else {
            index.saturating_sub(1)
        };
        if self.game.go(next) {
            self.route = None;
            self.persist();
            Change::Full
        } else {
            Change::None
        }
    }

    fn tap(&mut self, x: f32, y: f32) -> Change {
        let width = primitive_app::WIDTH as i32;
        if back_button(width).contains(x, y) {
            return Change::Exit;
        }
        if dispatch(width).contains(x, y) {
            self.route = Some(self.game.dispatch());
            self.persist();
            return Change::Partial;
        }
        if previous(width).contains(x, y) {
            return self.page(false);
        }
        if next(width).contains(x, y) {
            return self.page(true);
        }
        if reset(width).contains(x, y) {
            self.game.reset();
            self.route = None;
            self.persist();
            return Change::Partial;
        }
        if let Some(point) = Yard::points().find(|&point| tile_rect(width, point).contains(x, y)) {
            if self.game.toggle(point) {
                self.route = None;
                self.persist();
                return Change::Partial;
            }
        }
        Change::None
    }

    fn draw_yard(&self, canvas: &mut Canvas, width: i32) {
        let yard = self.game.yard();
        let w = UNIT * COLS as i32;
        let h = UNIT * ROWS as i32;
        canvas.card(screen_rect(width, LEFT - 18, TOP - 18, w + 36, h + 36));
        for col in 1..COLS {
            canvas.fill(
                screen_rect(width, LEFT + col as i32 * UNIT, TOP, 2, h),
                HAIRLINE,
            );
        }
        for row in 1..ROWS {
            canvas.fill(
                screen_rect(width, LEFT, TOP + row as i32 * UNIT, w, 2),
                HAIRLINE,
            );
        }
        for point in Yard::points() {
            let bounds = tile_rect(width, point);
            match yard.tile(point) {
                Tile::Rail(a, b) => {
                    draw_arm(canvas, bounds, a, BLACK, true);
                    draw_arm(canvas, bounds, b, BLACK, true);
                    centre_dot(canvas, bounds, 20, BLACK);
                }
                Tile::Junction(common) => {
                    let hit = bounds.inset(canvas.px(14));
                    let pressed = canvas.control_pressed(hit, false);
                    canvas.panel(hit, 22, 4, if pressed { BLACK } else { PAPER });
                    let ink = if pressed { PAPER } else { BLACK };
                    let selected = self.game.branch(point, common);
                    draw_arm(canvas, bounds, common, ink, true);
                    for branch in common.branches() {
                        let active = branch == selected;
                        draw_arm(
                            canvas,
                            bounds,
                            branch,
                            if active { ink } else { SECONDARY },
                            active,
                        );
                    }
                    centre_dot(canvas, bounds, 35, ink);
                }
                Tile::Start(heading) => draw_arm(canvas, bounds, heading, BLACK, true),
                Tile::Depot | Tile::Buffer => {
                    let directions = [
                        Direction::North,
                        Direction::East,
                        Direction::South,
                        Direction::West,
                    ];
                    for direction in directions {
                        let from = direction.opposite();
                        let connected = point.step(direction).is_some_and(|neighbour| {
                            match yard.tile(neighbour) {
                                Tile::Rail(a, b) => a == from || b == from,
                                Tile::Junction(common) => {
                                    common == from || common.branches().contains(&from)
                                }
                                Tile::Start(heading) => heading == from,
                                _ => false,
                            }
                        });
                        if connected {
                            draw_arm(canvas, bounds, direction, BLACK, true);
                        }
                    }
                }
                Tile::Empty => {}
            }
        }
        if let Some(route) = &self.route {
            for leg in route.points.windows(2) {
                let a = tile_rect(width, leg[0]);
                let b = tile_rect(width, leg[1]);
                let ax = a.x + a.w / 2;
                let bx = b.x + b.w / 2;
                let ay = a.y + a.h / 2;
                let by = b.y + b.h / 2;
                let thick = canvas.px(11).max(2);
                canvas.fill(
                    Rect::new(
                        ax.min(bx) - thick / 2,
                        ay.min(by) - thick / 2,
                        (ax - bx).abs() + thick,
                        (ay - by).abs() + thick,
                    ),
                    BLACK,
                );
            }
        }
        for point in Yard::points() {
            let r = tile_rect(width, point);
            match yard.tile(point) {
                Tile::Start(_) => station(canvas, r, "S", true),
                Tile::Depot => station(canvas, r, "D", self.outcome() == Some(Outcome::Delivered)),
                Tile::Buffer => station(
                    canvas,
                    r,
                    "×",
                    self.route
                        .as_ref()
                        .is_some_and(|route| route.points.last() == Some(&point)),
                ),
                _ => {}
            }
        }
    }
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        let width = canvas.bounds().w;
        header(canvas, "Switchyard", "Exit");
        canvas.text(
            screen_rect(width, 40, 158, 750, 55),
            self.game.yard().title,
            BODY.bold(),
        );
        canvas.text(
            screen_rect(width, 720, 161, 504, 50),
            &format!(
                "Yard {} of {}  ·  {} delivered",
                self.game.current() + 1,
                YARDS.len(),
                self.game.completed()
            ),
            CAPTION.right(),
        );
        canvas.text(
            screen_rect(width, 40, 220, 1184, 45),
            "S   Start     ·     D   Depot     ·     ×   Buffer",
            CAPTION.center(),
        );
        self.draw_yard(canvas, width);
        let message = if self.save_failed {
            "Could not save progress. Check the Kindle storage."
        } else {
            match self.outcome() {
                Some(Outcome::Delivered) => "Delivered. The train reached the depot.",
                Some(Outcome::Buffer) => "Stopped at a buffer. Try a different route.",
                Some(Outcome::Blocked) => "The tracks do not connect. Try another junction.",
                Some(Outcome::Loop) => "The train returned to the same track.",
                None => "Tap a junction to change the track, then send the train.",
            }
        };
        canvas.text(
            screen_rect(width, 40, 1294, 1184, 55),
            message,
            BODY.center(),
        );
        canvas.button(dispatch(width), "Dispatch", true, false);
        if self.game.current() > 0 {
            canvas.button(previous(width), "Previous", false, false);
        }
        canvas.button(reset(width), "Reset points", false, false);
        if self.game.current() + 1 < YARDS.len() {
            canvas.button(
                next(width),
                "Next",
                self.outcome() == Some(Outcome::Delivered),
                false,
            );
        }
    }

    fn input(&mut self, input: Input) -> Change {
        match input {
            Input::Tap(x, y) => self.tap(x, y),
            Input::PageBack => self.page(false),
            Input::PageForward => self.page(true),
            _ => Change::None,
        }
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.path {
            primitive_app::save(path, &serde_json::to_vec(&self.game)?)?;
        }
        Ok(())
    }
}

fn tile_rect(width: i32, point: Point) -> Rect {
    screen_rect(
        width,
        LEFT + point.x as i32 * UNIT,
        TOP + point.y as i32 * UNIT,
        UNIT,
        UNIT,
    )
}

fn draw_arm(canvas: &mut Canvas, r: Rect, direction: Direction, ink: u8, active: bool) {
    let x = r.x + r.w / 2;
    let y = r.y + r.h / 2;
    let from = if active { 0 } else { canvas.px(26) };
    let stroke = canvas.px(if active { 6 } else { 3 }).max(1);
    let rail = canvas.px(10);
    let vertical = direction == Direction::North || direction == Direction::South;
    let backwards = direction == Direction::North || direction == Direction::West;
    let span = if vertical { r.h } else { r.w } / 2;
    let coords = |start: i32, end: i32, side: i32, thickness: i32| {
        if vertical {
            Rect::new(
                x + side,
                if backwards { y - end } else { y + start },
                thickness,
                end - start,
            )
        } else {
            Rect::new(
                if backwards { x - end } else { x + start },
                y + side,
                end - start,
                thickness,
            )
        }
    };
    if active {
        for side in [-rail, rail - stroke] {
            canvas.fill(coords(from, span + 1, side, stroke), ink);
        }
        for start in (canvas.px(20)..span).step_by(canvas.px(22).max(1) as usize) {
            canvas.fill(
                coords(
                    start,
                    (start + stroke).min(span),
                    -rail - stroke,
                    (rail + stroke) * 2,
                ),
                ink,
            );
        }
    } else {
        for start in (from..span).step_by(canvas.px(17).max(1) as usize) {
            canvas.fill(
                coords(start, (start + canvas.px(8)).min(span), -stroke / 2, stroke),
                ink,
            );
        }
    }
}

fn centre_dot(canvas: &mut Canvas, r: Rect, diameter: i32, ink: u8) {
    let d = canvas.px(diameter).max(1);
    canvas.circle(
        Rect::new(r.x + (r.w - d) / 2, r.y + (r.h - d) / 2, d, d),
        ink,
    );
}

fn station(canvas: &mut Canvas, r: Rect, label: &str, active: bool) {
    let d = canvas.px(62).max(1);
    let bounds = Rect::new(r.x + (r.w - d) / 2, r.y + (r.h - d) / 2, d, d);
    canvas.panel(bounds, 12, 3, if active { BLACK } else { PAPER });
    canvas.text(
        bounds.inset(canvas.px(4)),
        label,
        TextStyle::new(35)
            .bold()
            .center()
            .ink(if active { PAPER } else { BLACK }),
    );
}

fn dispatch(width: i32) -> Rect {
    screen_rect(width, 265, 1370, 734, 100)
}
fn previous(width: i32) -> Rect {
    screen_rect(width, 40, 1515, 365, 100)
}
fn reset(width: i32) -> Rect {
    screen_rect(width, 449, 1515, 365, 100)
}
fn next(width: i32) -> Rect {
    screen_rect(width, 859, 1515, 365, 100)
}
