use crate::error::*;
use crate::event::Event;
use std::sync::mpsc::*;
use std::thread;
use terminal_size::{terminal_size, Height, Width};

struct Term {
	pub width: u16,
	pub height: u16,
	pub resized: bool,
	pub _w: u16,
	pub _h: u16,
	pub fg: String,
	pub bg: String,
	pub hide_cursor: String,
	pub show_cursor: String,
	pub alt_screen: String,
	pub normal_screen: String,
	pub clear: String,
	pub mouse_on: String,
	pub mouse_off: String,
	pub mouse_direct_on: String,
	pub mouse_direct_off: String,
	pub winch: Sender<Event>,
}
impl Term {
	///Updates width and height and sets resized flag if terminal has been resized
	pub fn refresh(&mut self, args: Vec<String>, force: bool) {
		if self.resized {
			self.winch.send(Event::Flag(true));
			return;
		}

		let term_size = terminal_size();
		match term_size {
			Some((Width(w), Height(h))) => {
				self._w = w;
				self._h = h;
			}
			None => throw_error("Unable to get size of terminal!"),
		};
	}
}
