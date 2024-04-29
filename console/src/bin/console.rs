use console::console_system::Console;
use sdl2::pixels::Color;

fn main() {
    let width = 1920;
    let height = 1080;
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    video.text_input().start();
    let window = video
        .window("Console", width, height)
        .resizable()
        .opengl()
        .build()
        .unwrap();
    let mut can = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())
        .expect("Error on canvas");
    let bg = sdl2::surface::Surface::load_bmp("./bg.bmp").unwrap();

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    let font = ttf_context.load_font("./font.ttf", 24).expect("test");
    let tc = can.texture_creator();
    let bg_tex = tc.create_texture_from_surface(bg).unwrap();

    let _text_surf = font
        .render("Hello, World!")
        .blended(Color::RGB(255, 255, 255))
        .unwrap();
    let mut e = sdl.event_pump().unwrap();
    let mut flash = 0;
    let tc_tex = can.texture_creator();
    let mut con = Console::new(25, 25, width as u32, height as u32, &tc_tex);
    con.set_text_color(Color::RGB(255, 255, 255));
    con.set_background(bg_tex);
    con.set_visible(true);
    con.start_shell();
    
    'main: loop {
        if con.handle_sdl_events(&mut e) == -1 {
            break 'main;
        }
        can.set_draw_color(Color::RGB(0, 0, 0));
        can.clear();
        flash += 1;
        let flash_on;
        if flash > 10 {
            flash_on = true;
            flash = 0;
        } else {
            flash_on = false;
        }
        con.draw(flash_on, &mut can, &tc, &font);
        can.present();
    }
    con.shutdown();
}
