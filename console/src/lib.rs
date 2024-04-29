// doesn't work right on MacOS
// works on Ubuntu

pub mod console_system {
    use logger::log::*;
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::rect::Rect;
    use sdl2::render::TextureQuery;
    use std::io::Read;
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    pub struct Console<'a> {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        text: String,
        input_text: String,
        line_height: usize,
        color: sdl2::pixels::Color,
        visible: bool,
        log: Log,
        background: Option<sdl2::render::Texture<'a>>,
        tc: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        child: Option<Child>,
        input_sender: Option<Sender<String>>,
        output_receiver: Option<Receiver<String>>,
        empty: bool,
    }

    pub fn printtext(
        can: &mut sdl2::render::Canvas<sdl2::video::Window>,
        tex: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        font: &sdl2::ttf::Font,
        x: i32,
        y: i32,
        color: sdl2::pixels::Color,
        text: &str,
    ) {
        let text_surf = font.render(text).blended(color).unwrap();
        let text_surf_tex = tex.create_texture_from_surface(&text_surf).unwrap();
        let TextureQuery {
            width: wi,
            height: hi,
            ..
        } = text_surf_tex.query();
        can.copy(
            &text_surf_tex,
            Some(Rect::new(0, 0, wi, hi)),
            Some(Rect::new(x, y, wi, hi)),
        )
        .expect("on font copy");
    }

    pub fn check_wrap(font: &sdl2::ttf::Font, x: i32, y: i32, w: u32, h: u32, text: &str) -> bool {
        let mut counter = 0;
        let mut ypos = y;
        let mut width = x;
        let metrics = font.find_glyph_metrics('A').unwrap();
        for ch in text.chars() {
            if (width + metrics.advance > (w - 25) as i32) || ch == '\n' {
                counter += 1;
                ypos += metrics.advance + metrics.maxy;
                width = x;
            } else {
                width += metrics.advance;
            }
        }
        let total_lines = h as i32/(metrics.advance as i32 +metrics.maxy as i32);
        if(counter > total_lines-2)
        {
            return true;
        }
        false
    }

    /// printtext width function for printing text to the screen aligned by a certain width
    pub fn printtext_width(
        blink: bool,
        line_height: &mut usize,
        can: &mut sdl2::render::Canvas<sdl2::video::Window>,
        tex: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        font: &sdl2::ttf::Font,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        color: sdl2::pixels::Color,
        text: &str,
    ) -> (i32, i32, i32) {
        if text.is_empty() {
            return (0, 0, 0);
        }

        let mut vlst: Vec<String> = Vec::new();
        let mut width = x;
        let metrics = font.find_glyph_metrics('A').unwrap();
        let mut value = String::new();
        let mut counter = 1;
        let mut first_line = 0;
        let mut index = 0;
        let mut ypos = y;

        for ch in text.chars() {
            if (width + metrics.advance > (w - 25) as i32) || ch == '\n' {
                counter += 1;
                if first_line == 0 {
                    first_line = index;
                }
                vlst.push(value);
                value = String::new();
                if ch != '\n' {
                    value.push(ch);
                }
                ypos += metrics.advance + metrics.maxy;
                width = x;
            } else {
                value.push(ch);
                width += metrics.advance;
            }
            index += 1;
        }

        if !value.is_empty() {
            vlst.push(value);
        }

        let mut yy = y;
        let mut line_index: usize = 0;

        for i in &vlst {
            if !i.is_empty() {
                printtext(can, tex, font, x, yy, color, i);
            }

            yy += metrics.advance + metrics.maxy;
            line_index += 1;
            if yy > h as i32 - 25 {
                *line_height = line_index;
                break;
            }
        }

        if blink {
            can.set_draw_color(color);
            can.fill_rect(Rect::new(
                width + 5,
                ypos,
                8,
                (metrics.maxy + metrics.advance) as u32,
            ))
            .expect("failed on rect");
        }

        let total_lines = h as i32 / (metrics.advance as i32 + metrics.maxy as i32);
        (counter, total_lines, first_line)
    }

    impl<'a> Console<'a> {
        /// create a new console
        pub fn new(
            xx: i32,
            yx: i32,
            wx: u32,
            hx: u32,
            tex: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        ) -> Console<'a> {
            let home_dir = dirs::home_dir();
            match home_dir {
                Some(hdir) => {
                    std::env::set_current_dir(hdir).expect("could not set directory");
                }
                None => {
                    println!("no home directory");
                }
            }

            let mut log_ = Log::new_file_log("console", "log.txt", true, true);
            log_.i("Console started up");

            Console {
                x: xx,
                y: yx,
                w: wx,
                h: hx,
                text: String::new(),
                input_text: String::new(),
                line_height: 27,
                color: sdl2::pixels::Color::RGB(255, 255, 255),
                visible: true,
                log: log_,
                background: None,
                tc: tex,
                child: None,
                input_sender: None,
                output_receiver: None,
                empty: false,
            }
        }

        pub fn shutdown(&mut self) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        pub fn start_shell(&mut self) {
            let (input_tx, input_rx) = mpsc::channel::<String>();
            let (output_tx, output_rx) = mpsc::channel::<String>();
            let output_tx = Arc::new(Mutex::new(output_tx));
            let mut child = Command::new("cmd.exe")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to start shell");

            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
          
            std::thread::spawn(move || {
                let mut stdin = stdin;
                for input in input_rx {
                    if writeln!(stdin, "{}", input).is_err() {
                        break;
                    }
                }
            });

            
            let stdout_tx = Arc::clone(&output_tx);
            std::thread::spawn(move || {
            	let mut stdout_reader = BufReader::new(stdout);
                let mut buffer = [0; 1];
                while let Ok(bytes_read) = stdout_reader.read(&mut buffer) {
                    if bytes_read == 0 {
                        break;
                    }
                    let mut tx = stdout_tx.lock().unwrap();
                    let mut s = String::new();
		    if(buffer[0] as char == '\r') { continue; }
                    s.push(buffer[0] as char);
                    tx.send(s)
                        .expect("failed to send stdout byte to main thread");
                }
            });
            
            let stderr_tx = Arc::clone(&output_tx);
            std::thread::spawn(move || {
                let mut stderr_reader = BufReader::new(stderr);
                let mut buffer = [0; 1];
                while let Ok(bytes_read) = stderr_reader.read(&mut buffer) {
                    if bytes_read == 0 {
                        break;
                    }
                    let mut tx = stderr_tx.lock().unwrap();
                    let mut s = String::new();
		    if(buffer[0] as char == '\r') { continue; }
                    s.push(buffer[0] as char);
                    tx.send(s)
                        .expect("failed to send stdout byte to main thread");
                }
            });
            self.child = Some(child);
            self.input_sender = Some(input_tx);
            self.output_receiver = Some(output_rx);
        }
        pub fn handle_sdl_events(&mut self, event_pump: &mut sdl2::EventPump) -> i32 {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => {
                        self.shutdown();
                        std::process::exit(0);
                    }
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Escape => {
                            return -1;
                        }
                        Keycode::Return => {
                            if let Some(ref sender) = self.input_sender {
                                    sender
                                        .send(self.input_text.clone())
                                        .expect("failed to send input");
                                    //self.text.push_str(&self.input_text);
                                    //self.text.push('\n');
                                    self.input_text.clear();
                            }
                        }
                        Keycode::Backspace => {
                            self.input_text.pop();
                        }
                        _ => {}
                    },
                    Event::TextInput { text, .. } => {
                        self.input_text.push_str(&text);
                    }
                    _ => {}
                }
            }

            if let Some(ref receiver) = self.output_receiver {
                loop {
                    match receiver.try_recv() {
                        Ok(output_byte) => {
                          self.text.push_str(&output_byte);   
                          print!("{}", output_byte);        
                        }
                        Err(e) => {
                            if e == std::sync::mpsc::TryRecvError::Empty {
                                break;
                            } else if e == std::sync::mpsc::TryRecvError::Disconnected {
                                println!("Channel disconnected");
                                return -1;
                            }
                        }
                    }
                }
            }
            0
        }

        pub fn set_background(&mut self, back: sdl2::render::Texture<'a>) {
            self.background = Some(back);
        }

        /// set console text color
        pub fn set_text_color(&mut self, col: sdl2::pixels::Color) {
            self.color = col;
        }

        /// set console is visible or not
        pub fn set_visible(&mut self, v: bool) {
            self.visible = v;
        }

        /// get console visible or not
        pub fn get_visible(&mut self) -> bool {
            self.visible
        }

        /// change console directory
        pub fn change_dir(&mut self, d: &str) {
            let result = std::env::set_current_dir(std::path::Path::new(d));
            match result {
                Ok(_) => {}
                Err(s) => {
                    self.println(&format!("\nError could not change directory... {}", s));
                }
            }
        }

        /// print text to the console
        pub fn print(&mut self, t: &str) {
            self.text.push_str(t);
        }

        /// print text to the console with trailing newline
        pub fn println(&mut self, t: &str) {
            self.text.push_str(t);
            self.text.push('\n');
        }

        /*
        fn execute_shell_command(&mut self, command: &str) {
            let mut child = Command::new("/bin/sh")
                .arg("-c")
                .arg(command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to execute shell");

            if let Some(ref mut stdin) = child.stdin.take() {
                if let Err(e) = writeln!(stdin, "{}", self.input_text) {
                    self.println(&format!("\nFailed to write to shell stdin: {}", e));
                }
            }

            let output = child.wait_with_output().expect("Failed to read shell output");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            self.println(&format!("\n{}", stdout));
            if !stderr.is_empty() {
                self.println(&format!("{}", stderr));
            }
        }*/
        /// draw the console
        pub fn draw(
            &mut self,
            blink: bool,
            can: &mut sdl2::render::Canvas<sdl2::video::Window>,
            tex: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
            font: &sdl2::ttf::Font,
        ) {
            if !self.visible {
                return;
            }
        
            match &self.background {
                Some(b) => {
                    let TextureQuery { width: wi, height: hi, .. } = b.query();
                    can.copy(&b, Some(Rect::new(0, 0, wi, hi)), Some(Rect::new(0, 0, self.w, self.h)))
                      .expect("on background copy");
                }
                None => {}
            }
        
            let mut total = String::new();
            total.push_str(&self.text);
            total.push_str(&self.input_text);
        
            printtext_width(
                blink,
                &mut self.line_height,
                can,
                tex,
                font,
                self.x,
                self.y,
                self.w,
                self.h,
                self.color,
                &total,
            );
           if check_wrap(font, self.x, self.y, self.w, self.h, &self.text) {
                if let Some(pos) = self.text.find('\n') {
                    self.text.drain(..=pos);
                    self.empty = true;
                }
            } else if self.empty == true {
                self.empty = false;
                /*if self.text.chars().count() >= 2 {
                    if let Some(ch) = self.text.chars().nth_back(1) {
                        if ch != '$' {
                            self.text.push('$');
                            self.text.push(' ');
                        }
                    }
                }*/
            }
        }
    }
}
