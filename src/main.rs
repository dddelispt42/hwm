/**
 * penrose :: example configuration
 *
 * penrose does not have a traditional configuration file and is not typically set up by patching
 * the source code: it is more like Xmonad or Qtile in the sense that it is really a library for
 * writing your own window manager. Below is an example main.rs that can serve as a template should
 * you decide to write your own WM using penrose.
 */
#[macro_use]
extern crate penrose;

use penrose::{
    contrib::{
        extensions::Scratchpad,
        hooks::{ActiveClientAsRootName, ClientSpawnRules, SpawnRule},
        layouts::paper,
    },
    core::{
        client::Client,
        config::Config,
        helpers::{index_selectors, spawn},
        hooks::Hook,
        layout::{bottom_stack, monocle, side_stack, Layout, LayoutConf},
        manager::WindowManager,
        ring::Selector,
        xconnection::XConn,
    },
    logging_error_handler,
    xcb::{XcbConnection, XcbHooks},
    Backward, Forward, Less, More, Result,
};

use simplelog::{LevelFilter, SimpleLogger};
use std::collections::HashMap;

// An example of a simple custom hook. In this case we are creating a NewClientHook which will
// be run each time a new client program is spawned.
struct MyClientHook {}
impl<X: XConn> Hook<X> for MyClientHook {
    fn new_client(&mut self, wm: &mut WindowManager<X>, c: &mut Client) -> Result<()> {
        wm.log(&format!("new client with WM_CLASS='{}'", c.wm_class()))
    }
}

fn main() -> Result<()> {
    // penrose will log useful information about the current state of the WindowManager during
    // normal operation that can be used to drive scripts and related programs. Additional debug
    // output can be helpful if you are hitting issues.
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default())
        .expect("failed to init logging");

    // Created at startup. See keybindings below for how to access them
    let mut config_builder = Config::default().builder();
    config_builder
        .workspaces(vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"])
        // Windows with a matching WM_CLASS will always float
        .floating_classes(vec!["dmenu", "dunst", "polybar", "rofi"])
        // Client border colors are set based on X focus
        .border_px(4)
        .gap_px(0)
        .top_bar(true)
        .bar_height(31)
        .focused_border(0xa2c000)
        .unfocused_border(0x3c3836);

    // When specifying a layout, most of the time you will want LayoutConf::default() as shown
    // below, which will honour gap settings and will not be run on focus changes (only when
    // clients are added/removed). To customise when/how each layout is applied you can create a
    // LayoutConf instance with your desired properties enabled.
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
        allow_wrapping: false,
    };

    // Default number of clients in the main layout area
    let n_main = 1;

    // Default percentage of the screen to fill with the main area of the layout
    let ratio = 0.6;

    // Layouts to be used on each workspace. Currently all workspaces have the same set of Layouts
    // available to them, though they track modifications to n_main and ratio independently.
    config_builder.layouts(vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[mono]", LayoutConf::default(), monocle, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
        Layout::floating("[----]"),
    ]);

    // Now build and validate the config
    let config = config_builder.build().unwrap();

    // NOTE: change these to programs that you have installed!
    let my_program_launcher = "rofi -show combi";
    let my_file_manager = "st -e lf";
    let my_terminal = "st";
    let my_browser = "brave";

    /* hooks
     *
     * penrose provides several hook points where you can run your own code as part of
     * WindowManager methods. This allows you to trigger custom code without having to use a key
     * binding to do so. See the hooks module in the docs for details of what hooks are avaliable
     * and when/how they will be called. Note that each class of hook will be called in the order
     * that they are defined. Hooks may maintain their own internal state which they can use to
     * modify their behaviour if desired.
     */
    let mut hooks: XcbHooks = vec![];
    hooks.push(Box::new(MyClientHook {}));

    // Using a simple contrib hook that takes no config. By convention, contrib hooks have a 'new'
    // method that returns a boxed instance of the hook with any configuration performed so that it
    // is ready to push onto the corresponding *_hooks vec.
    hooks.push(ActiveClientAsRootName::new());

    // Here we are using a contrib hook that requires configuration to set up a default workspace
    // on workspace "9". This will set the layout and spawn the supplied programs if we make
    // workspace "9" active while it has no clients.
    // hooks.push(DefaultWorkspace::new(
    //     "1",
    //     "[side]",
    //     vec!["st -c \"st - heiko@ed\" -T \"st - heiko@ed\""],
    // ));
    // hooks.push(DefaultWorkspace::new(
    //     "2",
    //     "[side]",
    //     vec!["st -c \"st - heiko@localhost\" -T \"st - heiko@localhost\""],
    // ));
    // hooks.push(DefaultWorkspace::new(
    //     "3",
    //     "[side]",
    //     vec!["st -c \"st - heiko@lab\" -T \"st - heiko@lab\""],
    // ));
    // hooks.push(DefaultWorkspace::new("4", "[side]", vec!["firefox"]));
    // hooks.push(DefaultWorkspace::new("5", "[side]", vec!["signal-desktop"]));
    // hooks.push(DefaultWorkspace::new("6", "[side]", vec![my_browser]));

    // spawn rules
    hooks.push(ClientSpawnRules::new(vec![
        SpawnRule::ClassName("Tor Browser", 0),
        SpawnRule::ClassName("brave-browser", 3),
        SpawnRule::ClassName("firefox", 3),
        SpawnRule::ClassName("gimp", 8),
        SpawnRule::ClassName("signal", 4),
        SpawnRule::ClassName("Thunderbird", 3),
        SpawnRule::ClassName("anki", 2),
        SpawnRule::WMName("st - heiko@ed", 1),
        SpawnRule::WMName("st - heiko@ed2", 1),
        SpawnRule::WMName("st - heiko@backup", 1),
        SpawnRule::WMName("st - heiko@localhost", 0),
    ]));

    // Scratchpad is an extension: it makes use of the same Hook points as the examples above but
    // additionally provides a 'toggle' method that can be bound to a key combination in order to
    // trigger the bound scratchpad client.
    let sp = Scratchpad::new("st", 0.8, 0.8);
    hooks.push(sp.get_hook());
    let sp2 = Scratchpad::new("st", 0.8, 0.8);
    hooks.push(sp2.get_hook());
    let sp3 = Scratchpad::new("st", 0.8, 0.8);
    hooks.push(sp3.get_hook());

    /* The gen_keybindings macro parses user friendly key binding definitions into X keycodes and
     * modifier masks. It uses the 'xmodmap' program to determine your current keymap and create
     * the bindings dynamically on startup. If this feels a little too magical then you can
     * alternatively construct a  HashMap<KeyCode, FireAndForget> manually with your chosen
     * keybindings (see helpers.rs and data_types.rs for details).
     * FireAndForget functions do not need to make use of the mutable WindowManager reference they
     * are passed if it is not required: the run_external macro ignores the WindowManager itself
     * and instead spawns a new child process.
     */
    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!(my_program_launcher);
        "M-d" => run_external!(my_program_launcher);
        "M-Return" => run_external!(my_terminal);
        "M-S-f" => run_external!(my_file_manager);
        "M-S-Return" => sp.toggle();
        "M-C-Return" => sp2.toggle();
        "M-A-Return" => sp3.toggle();

        // client management
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-q" => run_internal!(kill_client);
        "M-f" => run_internal!(toggle_client_fullscreen, &Selector::Focused);

        // applications
        "M-S-q" => run_external!("exit_menu");
        // "M-c" => run_external!("st -e clipmenu");
        "M-c" => run_external!("CM_LAUNCHER=rofi clipmenu");
        "M-w" => run_external!(my_browser);
        "M-b" => run_external!("bluetooth_menu");
        "M-m" => run_external!("pulsemixer");
        "M-period" => run_external!("rofimenu");
        "M-S-period" => run_external!("nerdfont_menu");
        "M-Print" => run_external!("screenshot_menu");
        "M-S-Print" => run_external!("screenshot_menu -s");
        // "M-S-w" => run_external!(format!("{} -e sudo nmtui", TERMINAL));
        // "M-r" => run_external!(format!("{} -e lf", TERMINAL));
        // "M-S-r" => run_external!(format!("{} -e htop", TERMINAL));

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace);
        "M-p" => run_internal!(cycle_screen, Forward);
        "M-n" => run_internal!(cycle_screen, Backward);
        "M-S-p" => run_internal!(drag_workspace, Forward);
        "M-S-n" => run_internal!(drag_workspace, Backward);

        // Layout management
        "M-l" => run_internal!(cycle_layout, Forward);
        "M-S-l" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => run_internal!(update_max_main, More);
        "M-A-Down" => run_internal!(update_max_main, Less);
        "M-A-Right" => run_internal!(update_main_ratio, More);
        "M-A-Left" => run_internal!(update_main_ratio, Less);

        "M-x" => run_internal!(detect_screens);
        "M-S-x" => run_external!("xrandr.sh");
        "M-A-Escape" => run_internal!(exit);

        refmap [ config.ws_range() ] in {
            "M-{}" => focus_workspace [ index_selectors(config.workspaces().len()) ];
            "M-S-{}" => client_to_workspace [ index_selectors(config.workspaces().len()) ];
        };

    };

    // The underlying connection to the X server is handled as a trait: XConn. XcbConnection is the
    // reference implementation of this trait that uses the XCB library to communicate with the X
    // server. You are free to provide your own implementation if you wish, see xconnection.rs for
    // details of the required methods and expected behaviour and xcb/xconn.rs for the
    // implementation of XcbConnection.
    let conn = XcbConnection::new()?;

    // Create the WindowManager instance with the config we have built and a connection to the X
    // server. Before calling grab_keys_and_run, it is possible to run additional start-up actions
    // such as configuring initial WindowManager state, running custom code / hooks or spawning
    // external processes such as a start-up script.
    let mut wm = WindowManager::new(config, conn, hooks, logging_error_handler());
    wm.init()?;

    // NOTE: If you are using the default XCB backend provided in the penrose xcb module, then the
    //       construction of the XcbConnection and resulting WindowManager can be done using the
    //       new_xcb_backed_window_manager helper function like so:
    //
    // let mut wm = new_xcb_backed_window_manager(config)?;

    spawn("/home/heiko/.config/dwm/autostart.sh")?;

    // grab_keys_and_run will start listening to events from the X server and drop into the main
    // event loop. From this point on, program control passes to the WindowManager so make sure
    // that any logic you wish to run is done before here!
    wm.grab_keys_and_run(key_bindings, HashMap::new())?;

    Ok(())
}
