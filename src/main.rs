//! penrose :: EWMH support
//!
//! It is possible to add EWMH support to penrose via an extension. This provides
//! information to external utilities such as panels and statusbars so that they
//! are able to interact with the window manager.
//!
//! `penrose::extensions::hooks::add_ewmh_hooks` can be used to compose the required
//! hooks into your existing Config before starting the window manager. If you want
//! to modify the support, each of the individual hooks can be found in
//! `penrose::extensions::hooks::ewmh`.
use penrose::{
    builtin::{
        actions::{
            exit,
            floating::{float_focused, reposition, resize, sink_all, sink_focused},
            key_handler, log_current_state, modify_with, send_layout_message, spawn,
        },
        hooks::SpacingHook,
        layout::{
            messages::{ExpandMain, IncMain, ShrinkMain},
            transformers::{Gaps, ReflectHorizontal, ReserveTop},
            CenteredMain, Grid, MainAndStack, Monocle,
        },
    },
    core::{
        bindings::{parse_keybindings_with_xmodmap, KeyEventHandler},
        layout::{Layout, LayoutStack},
        Config, State, WindowManager,
    },
    extensions::{
        hooks::{
            add_ewmh_hooks, add_named_scratchpads,
            manage::{FloatingCentered, SetWorkspace},
            NamedScratchPad, SpawnOnStartup, ToggleNamedScratchPad,
        },
        layout::{Conditional, Fibonacci, Tatami},
    },
    manage_hooks, map, stack,
    x::{query::ClassName, XConn, XConnExt},
    x11rb::RustConn,
    Xid,
};
use std::collections::HashMap;
use tracing_subscriber::{self, reload::Handle, EnvFilter};

pub type KeyHandler = Box<dyn KeyEventHandler<RustConn>>;

pub const FONT: &str = "FiraCode Nerd Font Mono";
pub const BLACK: u32 = 0x282828ff;
pub const WHITE: u32 = 0xebdbb2ff;
pub const GREY: u32 = 0x3c3836ff;
pub const BLUE: u32 = 0x458588ff;

pub const MAX_MAIN: u32 = 1;
pub const RATIO: f32 = 0.6;
pub const RATIO_STEP: f32 = 0.1;
pub const OUTER_PX: u32 = 5;
pub const INNER_PX: u32 = 5;
pub const BAR_HEIGHT_PX: u32 = 18;
pub const MAX_ACTIVE_WINDOW_CHARS: usize = 50;

pub const DEBUG_ENV_VAR: &str = "PENROSE_DEBUG";

// Delta for moving / resizing floating windows
const DELTA: i32 = 10;

pub const MON_1: &str = "eDP-1";
pub const MON_2: &str = "HDMI-2";

struct StickyClientState(Vec<Xid>);

pub fn add_sticky_client_state<X>(mut wm: WindowManager<X>) -> WindowManager<X>
where
    X: XConn + 'static,
{
    wm.state.add_extension(StickyClientState(Vec::new()));
    // wm.state.config.compose_or_set_refresh_hook(refresh_hook);

    wm
}

// fn refresh_hook<X: XConn>(state: &mut State<X>, x: &X) -> Result<()> {
//     let s = state.extension::<StickyClientState>()?;
//     let t = state.client_set.current_tag().to_string();
//     let mut need_refresh = false;
//
//     // clear out any clients we were tracking that are no longer in state
//     s.borrow_mut().0.retain(|id| state.client_set.contains(id));
//
//     for client in s.borrow().0.iter() {
//         if state.client_set.tag_for_client(client) != Some(&t) {
//             state.client_set.move_client_to_tag(client, &t);
//             need_refresh = true;
//         }
//     }
//
//     // we guard against refreshing only when clients were on the wrong screen
//     // so that we don't get into an infinite loop from calling refresh from
//     // inside of a refresh hook
//     if need_refresh {
//         x.refresh(state)?;
//     }
//
//     Ok(())
// }

pub fn toggle_sticky_client() -> KeyHandler {
    key_handler(|state, x: &RustConn| {
        let _s = state.extension::<StickyClientState>()?;
        let mut s = _s.borrow_mut();

        if let Some(&id) = state.client_set.current_client() {
            if s.0.contains(&id) {
                s.0.retain(|&elem| elem != id);
            } else {
                s.0.push(id);
            }

            drop(s);
            x.refresh(state)?;
        }

        Ok(())
    })
}

// Generate a raw key binding map in terms of parsable string key bindings rather than resolved key codes
pub fn raw_key_bindings<L, S>(
    toggle_scratch: ToggleNamedScratchPad,
    toggle_scratch_py: ToggleNamedScratchPad,
    handle: Handle<L, S>,
) -> HashMap<String, KeyHandler>
where
    L: From<EnvFilter> + 'static,
    S: 'static,
{
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_owned();

        // Windows
        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-space" => modify_with(|cs| cs.swap_focus_and_head()),
        "M-C-space" => modify_with(|cs| cs.rotate_focus_to_head()),
        "M-q" => modify_with(|cs| cs.kill_focused()),

        // Workspaces
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-S-bracketright" => modify_with(|cs| cs.drag_workspace_forward()),
        "M-S-bracketleft" => modify_with(|cs| cs.drag_workspace_backward()),

        // Layouts
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-S-Up" => send_layout_message(|| IncMain(1)),
        "M-S-Down" => send_layout_message(|| IncMain(-1)),
        "M-S-Right" => send_layout_message(|| ExpandMain),
        "M-S-Left" => send_layout_message(|| ShrinkMain),

        // Launchers
        "M-Print" => spawn("screenshot_menu"),
        "M-S-Print" => spawn("screenshot_menu -s"),
        "M-S-f" => spawn("st -e lf"),
        "M-c" => spawn("CM_LAUNCHER=rofi clipmenu"),
        "M-w" => spawn("qutebrowser"),
        "M-b" => spawn("bluethooth_menu"),
        "M-m" => spawn("st -e termusic"),
        "M-a" => spawn("rofi-pass"),
        "M-n" => spawn("st -e news"),
        "M-S-t" => spawn("st -e btop"),
        "M-S-x" => spawn("xrandr_menu"),
        "M-t" => spawn("term_menu"),
        "M-period" => spawn("rofimenu"),
        "M-S-period" => spawn("nerdfont_menu"),
        "M-semicolon" => spawn("rofi -show run"),
        "M-S-q" => spawn("exit_menu"),
        "M-d" => spawn("rofi -show run"),
        "M-Return" => spawn("st"),
        "M-A-w" => spawn("floating-webcam"),
        "M-S-Return" => Box::new(toggle_scratch),
        "M-C-Return" => Box::new(toggle_scratch_py),

        // Session management
        "M-A-l" => spawn("xflock4"),
        // "M-A-Escape" => power_menu(),

        "M-C-t" => toggle_sticky_client(),

        // Floating management
        "M-C-f" => float_focused(),
        "M-C-s" => sink_focused(),
        "M-C-S-s" => sink_all(),
        // Floating resize
        "M-C-Right" => resize(DELTA, 0),
        "M-C-Left" => resize(-DELTA, 0),
        "M-C-Up" => resize(0, -DELTA),
        "M-C-Down" => resize(0, DELTA),
        // Floating position
        "M-C-l" => reposition(DELTA, 0),
        "M-C-h" => reposition(-DELTA, 0),
        "M-C-k" => reposition(0, -DELTA),
        "M-C-j" => reposition(0, DELTA),

        // Debugging
        // "M-A-t" => set_tracing_filter(handle),
        "M-A-d" => log_current_state(),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |client_set| client_set.pull_tag_to_screen(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

fn layouts() -> LayoutStack {
    stack!(
        flex_tall(),
        flex_wide(),
        MainAndStack::side(MAX_MAIN, RATIO, RATIO_STEP),
        ReflectHorizontal::wrap(MainAndStack::side(MAX_MAIN, RATIO, RATIO_STEP)),
        MainAndStack::bottom(MAX_MAIN, RATIO, RATIO_STEP),
        Tatami::boxed(RATIO, RATIO_STEP),
        Fibonacci::boxed(MAX_MAIN, RATIO, RATIO_STEP),
        Grid::boxed(),
        Monocle::boxed()
    )
    .map(|layout| ReserveTop::wrap(Gaps::wrap(layout, OUTER_PX, INNER_PX), BAR_HEIGHT_PX))
}

fn flex_tall() -> Box<dyn Layout> {
    Conditional::boxed(
        "FlexTall",
        MainAndStack::side_unboxed(MAX_MAIN, RATIO, RATIO_STEP, false),
        CenteredMain::vertical_unboxed(MAX_MAIN, RATIO, RATIO_STEP),
        |_, r| r.w <= 1400,
    )
}

fn flex_wide() -> Box<dyn Layout> {
    Conditional::boxed(
        "FlexWide",
        MainAndStack::bottom_unboxed(MAX_MAIN, RATIO, RATIO_STEP, false),
        CenteredMain::horizontal_unboxed(MAX_MAIN, RATIO, RATIO_STEP),
        |_, r| r.w <= 1400,
    )
}

fn main() -> anyhow::Result<()> {
    // NOTE: Setting up tracing with dynamic filter updating inline as getting the type for
    // the reload Handle to work is a massive pain... this really should be in its own method
    // somewhere as the example here: https://github.com/tokio-rs/tracing/blob/master/examples/examples/tower-load.rs
    // _really_ seems to show that Handle only has a single type param, but when I try it in here
    // it complains about needing a second (phantom data) param as well?
    let tracing_builder = tracing_subscriber::fmt()
        // .json() // JSON logs
        // .flatten_event(true)
        .with_env_filter("info")
        .with_filter_reloading();

    let reload_handle = tracing_builder.reload_handle();
    // tracing_builder.finish().init();

    let startup_hook = SpawnOnStartup::boxed("/usr/local/scripts/penrose-startup.sh");
    let manage_hook = manage_hooks![
        ClassName("floatTerm") => FloatingCentered::new(0.8, 0.6),
        ClassName("Xnest") => FloatingCentered::new(0.8, 0.6),
        ClassName("copyq") => FloatingCentered::new(0.8, 0.6),
        ClassName("dmenu") => FloatingCentered::new(0.8, 0.6),
        ClassName("dunst") => FloatingCentered::new(0.8, 0.6),
        ClassName("onboard") => FloatingCentered::new(0.8, 0.6),
        ClassName("pinentry-gtk-2") => FloatingCentered::new(0.8, 0.6),
        ClassName("polybar") => FloatingCentered::new(0.8, 0.6),
        ClassName("floatTerm") => FloatingCentered::new(0.8, 0.6),
        ClassName("rofi")  => SetWorkspace("9"),
    ];
    let layout_hook = SpacingHook {
        inner_px: INNER_PX,
        outer_px: OUTER_PX,
        top_px: BAR_HEIGHT_PX,
        bottom_px: 0,
    };

    let config = add_ewmh_hooks(Config {
        default_layouts: layouts(),
        floating_classes: vec!["mpv-float".to_owned()],
        manage_hook: Some(manage_hook),
        startup_hook: Some(startup_hook),
        layout_hook: Some(Box::new(layout_hook)),
        ..Config::default()
    });

    // Create a new named scratchpad and toggle handle for use in keybindings.
    let (nsp, toggle_scratch) = NamedScratchPad::new(
        "terminal",
        "st -c StScratchpad",
        ClassName("StScratchpad"),
        FloatingCentered::new(0.8, 0.8),
        true,
    );

    let (nsp_py, toggle_scratch_py) = NamedScratchPad::new(
        "qt-console",
        "jupyter-qtconsole",
        ClassName("jupyter-qtconsole"),
        FloatingCentered::new(0.8, 0.8),
        true,
    );

    let conn = RustConn::new()?;
    let raw_bindings = raw_key_bindings(toggle_scratch, toggle_scratch_py, reload_handle);
    let key_bindings = parse_keybindings_with_xmodmap(raw_bindings)?;

    // Initialise the required state extension and hooks for handling the named scratchpad
    let wm = add_sticky_client_state(add_named_scratchpads(
        WindowManager::new(config, key_bindings, HashMap::new(), conn)?,
        vec![nsp, nsp_py],
    ));

    wm.run()?;

    Ok(())
}

