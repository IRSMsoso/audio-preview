use ratatui::crossterm::event::{poll, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, ExecutableCommand};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Gauge, HighlightSpacing, List, ListItem, ListState, Paragraph,
};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{read_dir, File};
use std::io;
use std::io::{stdout, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct App {
    current_path: PathBuf,

    current_path_directories: Vec<PathBuf>,
    current_path_directories_state: ListState,

    current_path_files: Vec<PathBuf>,
    current_path_files_state: ListState,

    should_exit: bool,

    perusing_files: bool,

    saved_directory_positions: HashMap<PathBuf, usize>,

    _stream: OutputStream,
    sink: Sink,
    current_source_total_duration: Option<Duration>,
    looping: bool,

    current_error_msg: Option<String>,
}

impl App {
    fn create_from_directory(directory: impl AsRef<Path>) -> Self {
        let (directories, files) = get_files_and_directories_in_directory(&directory);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&stream_handle).unwrap();

        Self {
            current_path: directory.as_ref().to_owned(),
            current_path_directories: directories,
            current_path_directories_state: ListState::default().with_selected(Some(0)),
            current_path_files: files,
            current_path_files_state: ListState::default().with_selected(Some(0)),
            should_exit: false,
            perusing_files: false,
            saved_directory_positions: Default::default(),
            _stream,
            sink,
            current_source_total_duration: None,
            looping: true,
            current_error_msg: None,
        }
    }

    fn enter_directory(&mut self) {
        let Some(selection_index) = self.current_path_directories_state.selected() else {
            return;
        };
        self.saved_directory_positions
            .insert(self.current_path.to_owned(), selection_index);

        self.current_path = self.current_path_directories[selection_index].clone();

        let (directories, files) = get_files_and_directories_in_directory(&self.current_path);

        self.current_path_directories = directories;
        self.current_path_files = files;

        match self.saved_directory_positions.get(&self.current_path) {
            Some(new_index) => {
                self.current_path_directories_state.select(Some(*new_index));
            }
            None => self.current_path_directories_state.select(Some(0)),
        }

        self.current_path_files_state.select(Some(0));
    }

    fn exit_directory(&mut self) {
        let Some(new_path) = self.current_path.parent() else {
            return;
        };

        if let Some(index) = self.current_path_directories_state.selected() {
            self.saved_directory_positions
                .insert(self.current_path.clone(), index);
        }

        self.current_path = new_path.to_owned();

        let (directories, files) = get_files_and_directories_in_directory(&self.current_path);

        self.current_path_directories = directories;
        self.current_path_files = files;

        match self.saved_directory_positions.get(&self.current_path) {
            Some(new_index) => {
                self.current_path_directories_state.select(Some(*new_index));
            }
            None => self.current_path_directories_state.select(Some(0)),
        }

        self.current_path_files_state.select(Some(0));
    }

    fn play_file(&mut self) -> anyhow::Result<()> {
        let Some(selection_index) = self.current_path_files_state.selected() else {
            return Ok(());
        };

        let Some(path_to_play) = self.current_path_files.get(selection_index) else {
            return Ok(());
        };

        self.sink.clear();

        let file = BufReader::new(File::open(path_to_play)?);

        let source = Decoder::new(file)?;

        self.current_source_total_duration = source.total_duration();

        if self.looping {
            self.sink.append(source.repeat_infinite());
        } else {
            self.sink.append(source);
        }

        self.sink.play();

        Ok(())
    }

    fn try_play_file(&mut self) {
        match self.play_file() {
            Ok(_) => {}
            Err(error) => self.current_error_msg = Some(error.to_string()),
        };
    }

    fn stop_playing(&mut self) {
        self.sink.clear();
        self.current_source_total_duration = None;
    }

    fn render_directories(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(self.current_path.to_str().unwrap_or("<ERROR>"))
            .borders(Borders::all())
            .border_set(symbols::border::ROUNDED)
            .border_style(if !self.perusing_files {
                Color::Blue
            } else {
                Color::White
            });

        let items: Vec<ListItem> = self
            .current_path_directories
            .iter()
            .map(|x| {
                let a = x
                    .file_name()
                    .unwrap_or("<ERROR>".as_ref())
                    .to_str()
                    .unwrap_or("<ERROR>");
                ListItem::new(Line::styled(a.to_owned(), Color::White))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.current_path_directories_state);
    }

    fn render_files(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(match self.looping {
                true => "Audio Files [LOOPING]",
                false => "Audio Files",
            })
            .borders(Borders::all())
            .border_set(symbols::border::ROUNDED)
            .border_style(if self.perusing_files {
                Color::Blue
            } else {
                Color::White
            });

        let items: Vec<ListItem> = self
            .current_path_files
            .iter()
            .map(|x| {
                let a = x
                    .file_name()
                    .unwrap_or("<ERROR>".as_ref())
                    .to_str()
                    .unwrap_or("<ERROR>");
                ListItem::new(Line::styled(a.to_owned(), Color::White))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.current_path_files_state);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(match &self.current_error_msg {
            Some(msg) => msg,
            None => "",
        })
        .centered()
        .render(area, buf);
    }

    fn render_progress_bar(&self, area: Rect, buf: &mut Buffer) {
        let current_progress = match self.current_source_total_duration {
            Some(duration) => ((self.sink.get_pos().as_secs_f64() / duration.as_secs_f64())
                % 1.0f64)
                .clamp(0.0f64, 1.0f64),
            None => 0.0f64,
        };

        Gauge::default()
            .block(Block::bordered())
            .gauge_style(Style::default())
            .use_unicode(true)
            .ratio(current_progress)
            .render(area, buf);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        self.current_error_msg = None;

        match key.code {
            KeyCode::Char('q') => self.should_exit = true,
            KeyCode::Char('l') => {
                self.looping = !self.looping;
                self.stop_playing();
            }
            _ => {}
        }

        match self.perusing_files {
            true => match key.code {
                KeyCode::Up => {
                    self.current_path_files_state.select_previous();
                    self.try_play_file();
                }
                KeyCode::Down => {
                    self.current_path_files_state.select_next();
                    self.try_play_file();
                }
                KeyCode::Right | KeyCode::Enter => self.try_play_file(),
                KeyCode::Tab => {
                    self.perusing_files = false;
                    self.stop_playing();
                }
                _ => {}
            },
            false => match key.code {
                KeyCode::Up => self.current_path_directories_state.select_previous(),
                KeyCode::Down => self.current_path_directories_state.select_next(),
                KeyCode::Left => self.exit_directory(),
                KeyCode::Right => self.enter_directory(),
                KeyCode::Tab => {
                    self.perusing_files = true;
                    self.try_play_file();
                }
                _ => {}
            },
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [main_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [directory_area, file_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(main_area);

        let [file_area, file_progress_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(file_area);

        self.render_directories(directory_area, buf);
        self.render_files(file_area, buf);
        self.render_footer(footer_area, buf);
        self.render_progress_bar(file_progress_area, buf);
    }
}

fn get_files_and_directories_in_directory(
    directory: impl AsRef<Path>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let directory_items = read_dir(directory).unwrap();

    let (directories, mut files): (Vec<PathBuf>, Vec<PathBuf>) = directory_items
        .flatten()
        .map(|x| x.path())
        .partition(|x| x.is_dir());

    files.retain(|x| {
        ["mp3", "wav", "ogg", ".flac"].contains(
            &match x.extension() {
                Some(extension) => match extension.to_str() {
                    Some(extension) => extension.to_lowercase(),
                    None => return false,
                },
                None => return false,
            }
            .as_str(),
        )
    });

    (directories, files)
}

pub(crate) fn run_interactive_mode() {
    let mut app = App::create_from_directory(&current_dir().unwrap());

    let mut terminal = init_terminal().unwrap();

    while !app.should_exit {
        terminal
            .draw(|f| f.render_widget(&mut app, f.area()))
            .unwrap();

        if !poll(Duration::from_millis(16)).unwrap() {
            continue;
        }

        if let Event::Key(key) = event::read().unwrap() {
            app.handle_key(key);
        }
    }

    restore_terminal().unwrap();
}

pub fn init_terminal() -> io::Result<Terminal<impl Backend>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore_terminal() -> io::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()
}
