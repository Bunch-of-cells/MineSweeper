use std::{
    borrow::{Borrow, Cow},
    time::{Duration, Instant},
};

use coffee::{
    graphics::{
        Color, Font, Frame, HorizontalAlignment, Mesh, Point, Shape, Text, VerticalAlignment,
        Window, WindowSettings,
    },
    input::{mouse::Button, Mouse},
    load::Task,
    Game, Result, Timer as CTimer,
};
use rand::distributions::{Bernoulli, Distribution};

const TOP_BAR: f32 = 200.0;
const DISTRIBUTION: f64 = 0.2;
const TILE_SIZE: usize = 50;

fn main() -> Result<()> {
    MineSweeper::run(WindowSettings {
        title: String::from("MineSweeper"),
        size: (500, 700),
        resizable: false,
        fullscreen: false,
        maximized: false,
    })
}

struct MineSweeper {
    tiles: Vec<Tile>,
    state: GameState,
    timer: Timer,
    height: usize,
    width: usize,
    tile_size: usize,
    distr: Bernoulli,
    mines: usize,
}

#[derive(Clone, Debug)]
struct Tile {
    mine: bool,
    neighbours: u8,
    revealed: bool,
    flagged: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum GameState {
    Running,
    Lose,
    Win,
    Menu,
}

enum Timer {
    None,
    Ticking(Instant),
    Stopped(Duration),
}

impl Timer {
    fn get_time(&self) -> Option<Duration> {
        match self {
            Timer::None => None,
            Timer::Ticking(s) => Some(s.elapsed()),
            Timer::Stopped(s) => Some(*s),
        }
    }

    fn stop(&mut self) {
        if let Self::Ticking(s) = *self {
            *self = Self::Stopped(s.elapsed())
        }
    }
}

impl MineSweeper {
    fn new(tile_size: usize, mut height: usize, mut width: usize, distr: f64) -> Self {
        height /= tile_size;
        width /= tile_size;
        let mut tiles = Vec::new();
        let distr = Bernoulli::new(distr).unwrap();
        for _ in 0..(height * width) {
            tiles.push(Tile {
                mine: distr.sample(&mut rand::thread_rng()),
                neighbours: 0,
                revealed: false,
                flagged: false,
            });
        }

        let mut game = Self {
            height,
            timer: Timer::None,
            width,
            state: GameState::Menu,
            tile_size,
            distr,
            mines: tiles.iter().filter(|t| t.mine).count(),
            tiles,
        };

        for tile in 0..game.tiles.len() {
            let neighbours = game
                .neighbours(tile)
                .iter()
                .filter(|t| game.tiles[**t].mine)
                .count() as u8;
            game.tiles[tile].neighbours = neighbours;
        }

        game
    }

    fn reset(&mut self) {
        let mut tiles = Vec::new();
        for _ in 0..(self.height * self.width) {
            tiles.push(Tile {
                mine: self.distr.sample(&mut rand::thread_rng()),
                neighbours: 0,
                revealed: false,
                flagged: false,
            });
        }

        self.mines = tiles.iter().filter(|t| t.mine).count();
        self.tiles = tiles;
        self.timer = Timer::None;
        self.state = GameState::Menu;

        for tile in 0..self.tiles.len() {
            let neighbours = self
                .neighbours(tile)
                .iter()
                .filter(|t| self.tiles[**t].mine)
                .count() as u8;
            self.tiles[tile].neighbours = neighbours;
        }
    }

    fn neighbours(&self, tile: usize) -> Vec<usize> {
        let mut neighbours = Vec::new();
        for vertical in -1..=1 {
            for horizontal in -1..=1 {
                if vertical == 0 && horizontal == 0 {
                    continue;
                }
                let col = tile / self.height;
                let row = tile % self.width;
                if row == 0 && horizontal == -1
                    || col == 0 && vertical == -1
                    || row == self.width - 1 && horizontal == 1
                    || col == self.height - 1 && vertical == 1
                {
                    continue;
                }

                let new_col = (col as i32 + vertical) as usize;
                let new_row = (row as i32 + horizontal) as usize;

                neighbours.push(new_col * self.height + new_row)
            }
        }
        neighbours
    }
}

impl Game for MineSweeper {
    type Input = Mouse;
    type LoadingScreen = ();

    fn load(window: &Window) -> Task<Self>
    where
        Self: Sized,
    {
        let height = (window.height() - TOP_BAR) as usize;
        let width = window.width() as usize;
        Task::succeed(move || Self::new(TILE_SIZE, height, width, DISTRIBUTION))
    }

    fn interact(&mut self, input: &mut Self::Input, _: &mut Window) {
        match self.state {
            GameState::Running => {
                if let [point, ..] = input.button_clicks(Button::Left) {
                    let tile = (point.y - TOP_BAR) as usize / self.tile_size * self.height
                        + point.x as usize / self.tile_size;
                    if let Tile {
                        mine,
                        revealed: r @ false,
                        flagged: false,
                        ..
                    } = &mut self.tiles[tile]
                    {
                        if *mine {
                            self.state = GameState::Lose;
                            self.timer.stop();
                        } else {
                            *r = true;
                            if self.tiles.iter().filter(|t| !t.mine).all(|t| t.revealed) {
                                self.state = GameState::Win;
                                self.timer.stop();
                            }
                        }
                    }
                } else if let [point, ..] = input.button_clicks(Button::Right) {
                    let tile = (point.y - TOP_BAR) as usize / self.tile_size * self.height
                        + point.x as usize / self.tile_size;
                    if let Tile {
                        revealed: false,
                        flagged,
                        ..
                    } = &mut self.tiles[tile]
                    {
                        *flagged = !*flagged;
                    }
                } else if let [point, ..] = input.button_clicks(Button::Middle) {
                    let tile = (point.y - TOP_BAR) as usize / self.tile_size * self.height
                        + point.x as usize / self.tile_size;
                    if let Tile {
                        revealed: true,
                        flagged: false,
                        mine: false,
                        neighbours,
                    } = self.tiles[tile]
                    {
                        let n = self.neighbours(tile);
                        if neighbours == n.iter().filter(|i| self.tiles[**i].flagged).count() as u8
                        {
                            for neighbour in n {
                                let Tile {
                                    mine,
                                    revealed,
                                    flagged,
                                    ..
                                } = &mut self.tiles[neighbour];
                                if *flagged {
                                    continue;
                                }
                                if *mine {
                                    self.state = GameState::Lose;
                                    self.timer.stop()
                                } else {
                                    *revealed = true;
                                }
                            }
                        }
                    }
                }
            }
            GameState::Lose | GameState::Win => {
                if input.is_button_pressed(Button::Middle) {
                    self.reset();
                }
            }
            GameState::Menu => {
                if input.is_button_pressed(Button::Middle) {
                    self.state = GameState::Running;
                    self.timer = Timer::Ticking(Instant::now());
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>, _: &CTimer) {
        frame.clear(Color::WHITE);
        match self.state {
            GameState::Running | GameState::Lose | GameState::Win => {
                let over = matches!(self.state, GameState::Lose | GameState::Win);

                let mut mesh = Mesh::new();
                for i in 0..=self.height {
                    let t = (i * self.tile_size) as f32 + TOP_BAR;
                    mesh.stroke(
                        Shape::Polyline {
                            points: vec![Point::new(0.0, t), Point::new(frame.width(), t)],
                        },
                        Color::BLACK,
                        2f32,
                    );
                }

                for i in 0..=self.width {
                    let t = (i * self.tile_size) as f32;
                    mesh.stroke(
                        Shape::Polyline {
                            points: vec![Point::new(t, TOP_BAR), Point::new(t, frame.height())],
                        },
                        Color::BLACK,
                        2f32,
                    );
                }

                let mut font = Font::load_from_bytes(include_bytes!("../font.ttf"))
                    .run(frame.gpu())
                    .unwrap();

                for (
                    i,
                    &Tile {
                        neighbours,
                        revealed,
                        flagged,
                        mine,
                    },
                ) in self.tiles.iter().enumerate()
                {
                    let (content, color) = if revealed && !flagged {
                        (Cow::Owned(neighbours.to_string()), Color::BLACK)
                    } else if flagged && mine && over {
                        (Cow::Borrowed("@"), Color::GREEN)
                    } else if flagged {
                        (Cow::Borrowed("F"), Color::BLUE)
                    } else if mine && over {
                        (Cow::Borrowed("@"), Color::RED)
                    } else {
                        (Cow::Borrowed(""), Color::BLACK)
                    };
                    font.add(Text {
                        content: content.borrow(),
                        position: Point::new(
                            (i % self.width * self.tile_size) as f32,
                            (i / self.height * self.tile_size) as f32 + TOP_BAR,
                        ),
                        bounds: (self.tile_size as f32, self.tile_size as f32),
                        size: self.tile_size as f32,
                        color,
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                    });
                }

                if over {
                    font.add(Text {
                        content: match self.state {
                            GameState::Lose => "You Lost",
                            GameState::Win => "You Win ",
                            _ => unreachable!(),
                        },
                        position: Point::new(frame.width() / 2.0 - 480.0, 20.0),
                        bounds: (960.0, 120.0),
                        size: 120f32,
                        color: Color::BLACK,
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                    });
                }

                let minecount = self.mines - self.tiles.iter().filter(|t| t.flagged).count();
                font.add(Text {
                    content: &format!("Minecount: {}", minecount),
                    position: Point::new(frame.width() / 2.0 - 260.0, TOP_BAR - 40.0),
                    bounds: (520.0, 40.0),
                    size: 40f32,
                    color: Color::BLACK,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                });

                let time = self.timer.get_time().unwrap();
                font.add(Text {
                    content: &format!("{}:{}", time.as_secs() / 60, time.as_secs() % 60),
                    position: Point::new(frame.width() / 2.0 - 150.0, TOP_BAR - 100.0),
                    bounds: (300.0, 60.0),
                    size: 60f32,
                    color: Color::BLACK,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                });

                mesh.draw(&mut frame.as_target());
                font.draw(&mut frame.as_target());
            }
            GameState::Menu => {
                let mut font = Font::load_from_bytes(include_bytes!("../font.ttf"))
                    .run(frame.gpu())
                    .unwrap();

                font.add(Text {
                    content: "Menu!",
                    position: Point::new((self.width / 2) as f32, (self.height / 2) as f32),
                    bounds: (frame.width(), frame.height()),
                    size: 120f32,
                    color: Color::BLACK,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                });
                font.draw(&mut frame.as_target());
            }
        }
    }
}
