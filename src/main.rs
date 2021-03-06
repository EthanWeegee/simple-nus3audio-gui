mod item_properties;
mod layout;
mod list;
mod playback;
mod rect;
mod settings;

use fltk::{
	prelude::*,
	app,
	dialog::{
		NativeFileChooser, FileDialogType
	},
	enums::{
		Cursor, Event, FrameType, Shortcut
	},
	menu::{
		MenuBar, MenuFlag
	},
	window::Window
};
use nus3audio::Nus3audioFile;
use std::fs;
use crate::{
	layout::alert,
	list::{
		List,
		ListItem
	},
	playback::Playback,
	settings::Settings
};

#[derive(Clone, Copy)]
pub enum Message {
	/// The window will re-lay itself out.
	ReLay,
	/// Clear the working nus3audio.
	New,
	/// Open a nus3audio.
	Open,
	/// Play.
	PlayPause,
	/// Stop the currently playing sound.
	Stop,
	/// Update the seek bar.
	Update,
	Seek,
	/// Save the working nus3audio.
	Save,
	/// Save the nus3audio to a new location.
	SaveAs,
	/// Export a single sound.
	ExportSingle,
	/// Export everything.
	ExportAll,
	/// Add a single sound.
	Add,
	/// Remove the selected sound.
	Remove,
	/// Open sound properties window.
	Properties,
	/// Replace a single sound.
	Replace,
	/// Configure the VGAudioCli path.
	ConfigureVGAudioCliPath,
	#[cfg(not(target_os = "windows"))]
	/// Configure the .NET runtime path.
	/// 
	/// Exclusive to not-Windows, because Windows (likely) doesn't need this.
	/// 
	/// Though the .NET runtime is not configurable here in Windows,
	/// the setting is still used there (although it defaults to an empty string).
	ConfigureRuntimePath,
	/// Configure the vgmstream path.
	ConfigureVgmstreamPath,
	/// Show the welcome message again.
	WelcomeGreeting,
	/// Open the online manual.
	Manual,
	/// Quit the application.
	Quit(i32)
}

const NAME: &str = "simple-nus3audio-gui";

fn main() {
	let app = app::App::default();
	let (s, r) = app::channel();
	let mut window = Window::new(0, 0, 250, 200, NAME);
	window.size_range(200, 150, 0, 0);

	// Menu
	let mut menu = MenuBar::default();
	menu.set_frame(FrameType::ThinUpBox);

	menu.add_emit(
		"&File/&New\t",
		Shortcut::Ctrl | 'n',
		MenuFlag::Normal,
		s,
		Message::New,
	);
	menu.add_emit(
		"&File/&Open nus3audio\t",
		Shortcut::Ctrl | 'o',
		MenuFlag::Normal,
		s,
		Message::Open,
	);
	menu.add_emit(
		"&File/&Save nus3audio\t",
		Shortcut::Ctrl | 's',
		MenuFlag::Normal,
		s,
		Message::Save,
	);
	menu.add_emit(
		"&File/Save nus3audio &as...\t",
		Shortcut::Ctrl | Shortcut::Shift | 's',
		MenuFlag::Normal,
		s,
		Message::SaveAs,
	);
	menu.add_emit(
		"&File/&Export single sound...\t",
		Shortcut::Ctrl | 'e',
		MenuFlag::Normal,
		s,
		Message::ExportSingle,
	);
	menu.add_emit(
		"&File/E&xport all...\t",
		Shortcut::Ctrl | Shortcut::Shift | 'e',
		MenuFlag::Normal,
		s,
		Message::ExportAll,
	);
	menu.add_emit(
		"&File/&Quit\t",
		Shortcut::Ctrl | 'q',
		MenuFlag::Normal,
		s,
		Message::Quit(0),
	);
	menu.add_emit(
		"&Edit/&Add sound\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::Add,
	);
	menu.add_emit(
		"&Edit/Re&move selected sound\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::Remove,
	);
	menu.add_emit(
		"&Edit/Sound &properties...\t",
		Shortcut::Ctrl | 'p',
		MenuFlag::Normal,
		s,
		Message::Properties,
	);
	menu.add_emit(
		"&Edit/&Replace single sound...\t",
		Shortcut::Ctrl | 'r',
		MenuFlag::Normal,
		s,
		Message::Replace,
	);
	menu.add_emit(
		"&Edit/&Configure VGAudioCli path...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::ConfigureVGAudioCliPath,
	);
	#[cfg(not(target_os = "windows"))]
	menu.add_emit(
		"&Edit/Configure .&NET runtime path...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::ConfigureRuntimePath,
	);
	menu.add_emit(
		"&Edit/&Configure vgmstream path...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::ConfigureVgmstreamPath,
	);
	menu.add_emit(
		"&Playback/&Play\t",
		Shortcut::from_char(' '),
		MenuFlag::Normal,
		s,
		Message::PlayPause,
	);
	menu.add_emit(
		"&Playback/&Stop\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::Stop,
	);
	menu.add_emit(
		"&Help/&VGAudioCli\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::WelcomeGreeting,
	);
	menu.add_emit(
		"&Help/User &manual...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::Manual,
	);

	// Playback
	let mut playback = Playback::new(s);

	// This will contain all the list items
	let mut file_list: List = List::new();

	let mut start_input = fltk::input::IntInput::default();
	start_input.set_tooltip("Loop start position in samples");
	let mut end_input = fltk::input::IntInput::default();
	end_input.set_tooltip("Loop end position in samples");

	window.make_resizable(true);
	window.end();
	window.show();

	// Now we need to lay the window out!
	{
		let (play_widget, slider_widget) = playback.get_widgets_mut();
		layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut())
	}

	window.handle(move |_, event| match event {
		Event::Resize => {
			s.send(Message::ReLay);
			true
		},
		_ => { false }
	});
	window.set_callback(move |_| {
		if app::event() == Event::Close {
			s.send(Message::Quit(0));
			app::program_should_quit(false)
		}
	});

	let mut settings = Settings::new_default();

	// Show the first-time greeting if necessary
	settings.first_time_greeting(&window, s);

	// Create the settings if needed
	if let Err(error) = Settings::create_settings() {
		// We won't exit in this case, but we'll probably have issues later
		fltk::dialog::message_title("Error");
		alert(&window, &format!("Error creating the settings directory:\n{}", error))
	}

	// And reset the cache
	if let Err(error) = Settings::reset_cache() {
		fltk::dialog::message_title("Fatal Error");
		alert(&window, &format!("Error creating the cache directory:\n{}", error));
		std::process::exit(1)
	}
	
	// Main event loop
	while app.wait() {
		// Handle events
		if let Some(e) = r.recv() {
			match e {
				Message::ReLay => {
					let (play_widget, slider_widget) = playback.get_widgets_mut();
					layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut())
				},
				Message::New => {
					file_list.clear()
				},
				Message::Open => {
					let mut file_dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
					file_dialog.set_filter("*.nus3audio");
					// Get file selection
					file_dialog.show();

					if file_dialog.filename().exists() {
						window.set_cursor(Cursor::Wait);

						// Attempt to read chosen file
						let raw = match std::fs::read(file_dialog.filename()) {
							Ok(r) => r,
							Err(e) => {
								fltk::dialog::message_title("Error");
								window.set_cursor(Cursor::Default);
								alert(&window, &format!("Error reading file:\n{}", e));
								continue
							}
						};

						// Try to load the nus3audio file
						let nus3audio = match Nus3audioFile::try_from_bytes(&raw) {
							Some(f) => f,
							None => {
								fltk::dialog::message_title("Error");
								window.set_cursor(Cursor::Default);
								alert(&window, "Error parsing file");
								continue
							}
						};

						// Stop current playback before loading the file into the list
						playback.stop_sink();

						file_list.clear();
						file_list.name = file_dialog.filename().file_name().unwrap().to_string_lossy().to_string();
						file_list.path = Some(file_dialog.filename());

						// Add the files to the list
						for file in nus3audio.files {
							let mut item = ListItem::new(file.name.clone());
							let mut item_name = file.name;

							let extension = list::extension_of_encoded(&file.data);

							// Set the item extension
							if let Ok(extension) = extension {
								item.extension = extension
							}

							if let Err(error) = item.from_encoded(&file_list.name, file.data, &settings) {
								fltk::dialog::message_title("Error");
								window.set_cursor(Cursor::Default);
								alert(&window, &format!("Could not decode {}:\n{}", item_name, error));
							};

							// Set the item extension in the name
							item_name.push_str(&format!(".{}", item.extension));

							file_list.add_item(item, &item_name);
						};

						file_list.redraw();
						window.set_cursor(Cursor::Default)
					}
				},
				Message::ExportSingle => {
					if let Some((index, sound_name)) = file_list.selected() {
						let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

						let (filter, default) = match list_item.extension {
							list::AudioExtension::Bin => ("*", "bin"),
							_ => ("*.wav\n*.idsp\n*.lopus", "wav")
						};

						// Make the default file name the sound's name, with ".wav" as the extension
						let default = std::path::PathBuf::from(&sound_name).with_extension(default);

						let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
						save_dialog.set_filter(filter);

						// Set the default file name to save
						if let Some(filename) = default.to_str() {
							save_dialog.set_preset_file(filename)
						}

						save_dialog.show();

						let target_file = save_dialog.filename();
						let extension = match target_file.extension() {
							Some(extension) => {
								match extension.to_str() {
									Some(extension) => {
										extension
									},
									_ => "wav"
								}
							},
							_ => "wav"
						};

						if !save_dialog.filename().to_string_lossy().is_empty() {
							window.set_cursor(Cursor::Wait);

							let target_file = target_file.with_extension(extension);

							let raw = if extension == "wav" {
								list_item.get_audio_wav(None)
							} else {
								list_item.get_nus3_encoded_raw(&file_list.name, extension, &settings)
							};

							if let Err(error) = raw {
								fltk::dialog::message_title("Error");
								window.set_cursor(Cursor::Default);
								alert(&window, &error.to_string());
								continue
							}

							if let Err(error) = fs::write(target_file, &raw.unwrap()) {
								fltk::dialog::message_title("Error");
								alert(&window, &error.to_string());
							}

							window.set_cursor(Cursor::Default)
						}
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
					
				},
				Message::ExportAll => {
					let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveDir);
					save_dialog.set_filter("*.wav");
					save_dialog.show();

					if !save_dialog.filename().to_string_lossy().is_empty() {
						window.set_cursor(Cursor::Wait);

						let mut skipped = String::new();
						let mut index: usize = 0;

						while let Some(sound_name) = file_list.get_label_of(index) {
							let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
							match list_item.get_audio_wav(None) {
								Ok(raw) => {
									let target_file = save_dialog.filename().join(&format!("{}.wav", sound_name));

									if let Err(error) = fs::write(target_file, raw) {
										fltk::dialog::message_title("Error");
										window.set_cursor(Cursor::Default);
										alert(&window, &format!("Error writing file:\n{}", error));
										break
									}
								},
								Err(error) => skipped.push_str(&format!("{}: {}\n", sound_name, error))
							}
							
							index += 1
						}

						if !skipped.is_empty() {
							fltk::dialog::message_title("Warning");
							alert(&window, &format!("The following items were skipped:\n{}", skipped))
						}

						window.set_cursor(Cursor::Default)
					}
				},
				Message::Add => {
					let item = ListItem::new(format!("new_sound_{}", file_list.items.len() + 1));
					file_list.add_item(item, &format!("new_sound_{}.idsp", file_list.items.len() + 1))
				},
				Message::Remove => {
					if let Some((index, _)) = file_list.selected() {
						file_list.remove(index)
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
				},
				Message::Properties => {
					let (index, name, extension) = if let Some((index, _)) = file_list.selected() {
						let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

						if item_properties::configure(list_item, &window) {
							// Item was modified
							file_list.modified = true
						}
						(index, list_item.name.clone(), list_item.extension.clone())
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
						continue
					};

					// Update the label of the item
					file_list.set_label_of(index, &format!("{}.{}", name, extension));

					// Update the progress slider in case we were playing anything
					playback.on_update()
				},
				Message::Replace => {
					if let Some((index, _)) = file_list.selected() {
						window.set_cursor(Cursor::Wait);
						if let Err(error) = file_list.replace(index, &settings) {
							fltk::dialog::message_title("Error");
							window.set_cursor(Cursor::Default);
							alert(&window, &error.to_string());
							continue
						}
						window.set_cursor(Cursor::Default);
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
				},
				Message::Save => {
					if file_list.path.is_some() {
						window.set_cursor(Cursor::Wait);
						if let Err(error) = file_list.save_nus3audio(None, &settings) {
							fltk::dialog::message_title("Error");
							window.set_cursor(Cursor::Default);
							alert(&window, &format!("Error saving file:\n{}", error));
							continue
						}

						window.set_cursor(Cursor::Default)
					} else {
						// Nothing to save to.
						s.send(Message::SaveAs)
					}
				},
				Message::SaveAs => {
					let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
					save_dialog.set_filter("*.nus3audio");
					save_dialog.show();

					if !save_dialog.filename().to_string_lossy().is_empty() {
						window.set_cursor(Cursor::Wait);
						if let Err(error) = file_list.save_nus3audio(Some(&save_dialog.filename()), &settings) {
							fltk::dialog::message_title("Error");
							window.set_cursor(Cursor::Default);
							alert(&window, &format!("Error saving file:\n{}", error))
						}
					}
				},
				Message::PlayPause => {
					if let Err(error) = playback.on_press(&mut file_list) {
						fltk::dialog::message_title("Error");
						alert(&window, &error);
					}
				},
				Message::Stop => playback.stop_sink(),
				Message::Update => playback.on_update(),
				Message::Seek => playback.on_seek(),
				Message::ConfigureVGAudioCliPath => settings.configure_vgaudio_cli_path(&window),
				#[cfg(not(target_os = "windows"))]
				Message::ConfigureRuntimePath => settings.configure_vgaudio_cli_prepath(&window),
				Message::ConfigureVgmstreamPath => settings.configure_vgmstream_path(&window),
				Message::WelcomeGreeting => {
					settings.set_first_time(true);
					settings.first_time_greeting(&window, s)
				},
				Message::Manual => { let _ = open::that("https://github.com/EthanWeegee/simple-nus3audio-gui/wiki/Usage-Manual"); },
				Message::Quit(code) => {
					// True if we should quit
					let response = if file_list.modified {
						fltk::dialog::message_title("Warning");
						let response = layout::choice2(&window, "You have currently unsaved changes.\nWould you still like to quit?", "Quit", "Go back", "");

						if let Some(0) = response {
							// Selected "Quit"
							true
						} else {
							// Selected "Go Back"
							false
						}
					} else {
						// Nothing unsaved
						true
					};

					if response {
						settings.save();
						Settings::reset_cache().expect("Failed to reset the cache directory");
						fltk::app::quit();
						std::process::exit(code)
					}
				}
			}
		}
	}

	settings.save();
	Settings::reset_cache().expect("Failed to reset the cache directory")
}
