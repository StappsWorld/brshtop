use crate::error::*;
use std::thread;
use std::sync::mpsc::*;
use terminal_size::{Width, Height, terminal_size};


pub enum Event {
    Flag(bool),
    Wait,
}

struct Term {
    pub width: u16,
	pub height: u16,
	pub resized: bool,
	pub _w : u16,
	pub _h : u16,
	pub fg: String,												
	pub bg: String, 
	pub hide_cursor: String, 
	pub show_cursor: String,
	pub alt_screen: String,
	pub normal_screen 	: String,
	pub clear: String,
	pub mouse_on: String,
	pub mouse_off: String,
	pub mouse_direct_on: String,
	pub mouse_direct_off: String,
	pub winch: Sender<Event>,
} impl Term {

	///Updates width and height and sets resized flag if terminal has been resized
	pub fn refresh(&mut self, args : Vec<String>, force : bool) {
		if self.resized {
			self.winch.send(Event::Flag(true));
			return;
		}

		let term_size = terminal_size();
		match term_size {
			Some((Width(w), Height(h))) => {
				self._w = w;
				self._h = h;
			},
			None => throw_error("Unable to get size of terminal!"),
		};
		

	}
}