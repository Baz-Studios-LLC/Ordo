//! Ordo — a small UI kit for Bevy games.
//!
//! Not a framework and not a general-purpose library. This is the set of
//! pieces that turned up, hand-written, in three separate games, lifted once so
//! they stop being written a fourth time. It is built for the games in this
//! repository's neighbourhood and it makes choices a public library could not:
//! one Bevy version, one layout idiom, no deprecation cycle, and freedom to
//! break between games because every game pins its own rev.
//!
//! Two ideas carry the whole thing.
//!
//! **The game owns the palette.** Ordo names roles — [`Role::PanelBg`],
//! [`Role::CardBorder`] — and the game says what color those are. A kit that
//! shipped its own palette would force a game like Divus Factus, whose panels
//! are tinted from the same ramps its villagers' clothes are dyed from, to keep
//! two sets of colors in step by hand forever.
//!
//! **Nothing is painted at spawn.** A widget carries tags saying which role and
//! which metric it follows; a pass fills them in afterward. That is what lets
//! the theme file be edited with the game running, which is the single largest
//! difference between this and hand-rolling it again.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use ordo::prelude::*;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"))
//!     .run();
//! ```

pub mod book;
pub mod overlay;
pub mod placard;
pub mod radial;
pub mod stepper;
pub mod tabs;
pub mod theme;
pub mod widgets;
pub mod window;

use bevy::prelude::*;

pub use book::{
    BookParts, ChapterButton, ChipParts, PageParts, adorn, book, chapter, chip, col, grid_row,
    page, plate,
};
pub use overlay::{
    Layer, Lifetime, Notice, Notices, Proclaimed, ProclaimedToken, Proclamation, ProclamationStage,
    Proclamations, Toast, ToastShelf, Tooltip, TooltipView, proclamation_stage, shelf, toast_shelf,
};
pub use placard::{Placard, PlacardParts, Rising, depth_scale, pin, placard};
pub use theme::{
    ColorSpec, Edge, Face, Fill, FontFace, FontRole, FontSpec, Ink, Metric, Opacity, Ramps, Role,
    Sheen, TextSize, Theme, ThemeAsset, ThemeHandle,
};
pub use widgets::{
    Anchor, Anchored, Hanging, InspectorPanel, LabelColumn, OrdoButton, Padded, Panel, RowHeight,
    backdrop, body, button, card, dim, hanging_rail, heading, inspector_panel, label, panel,
    readout, row, rule, spring,
};

pub mod prelude {
    pub use crate::OrdoPlugin;
    pub use crate::overlay::{Layer, Lifetime, Notice, Notices, Tooltip, shelf, toast_shelf};
    pub use crate::placard::{Placard, PlacardParts, Rising, depth_scale, pin, placard};
    pub use crate::radial::{Radial, RadialArt, Spent, Wedge, radial, wedge};
    pub use crate::stepper::{StepperParts, stepper};
    pub use crate::tabs::{Pane, Selected, Tab, Tabs, pane, tab, tab_strip};
    pub use crate::theme::{
        Edge, Face, Fill, FontRole, Ink, Metric, Opacity, Ramps, Role, TextSize, Theme,
    };
    pub use crate::widgets::{
        Anchor, InspectorPanel, backdrop, body, button, card, dim, heading, inspector_panel, label,
        panel, row, rule, spring,
    };
    pub use crate::window::{CloseButton, DragHandle, Titled, window};
}

/// Whether there is an image collection to add generated art to.
///
/// The radial menu draws its own wheel, which means writing to `Assets<Image>` — and a
/// system that asks for a resource which does not exist is an error, not a no-op. Ordo's
/// suite runs headless, where the asset plugins are absent, so this is asked rather than
/// assumed. Same reasoning as [`the_pointer_exists`].
fn the_images_exist(images: Option<Res<Assets<Image>>>) -> bool {
    images.is_some()
}

/// Whether there is a mouse and a window to ask about.
///
/// False in a headless test, where the input plugins are not installed - and a
/// system that asks for a resource which does not exist is an error, not a
/// no-op.
fn the_pointer_exists(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    primary: Query<(), With<bevy::window::PrimaryWindow>>,
) -> bool {
    buttons.is_some() && !primary.is_empty()
}

/// Installs the theme, its file, and the passes that paint.
#[derive(Default)]
pub struct OrdoPlugin {
    theme_path: Option<String>,
}

impl OrdoPlugin {
    /// No theme file. [`Theme`] keeps its defaults, and the game is free to
    /// write them itself — which is the right shape for a game whose colors
    /// are computed rather than chosen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load and watch a theme file, relative to the asset root.
    pub fn with_theme(path: impl Into<String>) -> Self {
        Self {
            theme_path: Some(path.into()),
        }
    }
}

/// Ordo's passes, so a game can order its own work against them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrdoSet;

impl Plugin for OrdoPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ThemeAsset>()
            .register_asset_loader(theme::ThemeLoader)
            .init_resource::<Theme>()
            .init_resource::<Ramps>()
            .init_resource::<Notices>()
            .init_resource::<overlay::Proclamations>()
            .init_resource::<overlay::HoverClock>()
            .init_resource::<window::Dragging>()
            .init_resource::<radial::RadialArt>()
            .add_systems(
                Update,
                (
                    // What is on screen, and how faded, before anything is
                    // painted — otherwise a toast spends its first frame at
                    // whatever opacity it was spawned with.
                    overlay::apply_layers,
                    placard::raise_placards,
                    placard::place_placards,
                    overlay::show_notices,
                    // Paired: the chain tuple is at Bevy's ceiling, and these
                    // two are one subject - the stage and its player.
                    (overlay::stage_proclamations, overlay::play_proclamations),
                    overlay::cap_shelf,
                    overlay::track_hover,
                    overlay::show_tooltips,
                    overlay::place_tooltips,
                    overlay::fade_tooltip_text,
                    overlay::age,
                    // Chrome BEFORE paint, so a title bar put on this frame is
                    // painted this frame rather than spending one bare.
                    window::dress_windows,
                    // These three want a mouse and a real window, and a kit
                    // that cannot be tested without one is a kit nobody tests.
                    // Ordo's own suite runs headless; so, now, does anyone
                    // else's.
                    window::drag_windows.run_if(the_pointer_exists),
                    window::close_windows,
                    window::focus_windows.run_if(the_pointer_exists),
                    // Then the theme, then the paint.
                    theme::apply_theme_asset,
                    theme::repaint,
                    widgets::relayout,
                    widgets::resize_rows,
                    widgets::paint_buttons,
                )
                    .chain()
                    .in_set(OrdoSet),
            )
            // Registered separately because the tuple above is exactly at Bevy's
            // twenty-system ceiling. Placed before painted, for the same reason
            // chrome is dressed before paint: otherwise a wedge spawned this frame
            // spends it at the origin in the wrong color.
            .add_systems(
                Update,
                (
                    radial::dress_radials.run_if(the_images_exist),
                    radial::place_wedges,
                    radial::paint_wedges,
                    radial::aim_highlight,
                    // Before the button paint, which reads the `Selected` this puts on.
                    tabs::space_strips,
                    tabs::paint_folder_tabs,
                    tabs::show_selected_pane,
                    tabs::shape_tabs,
                    stepper::size_steppers,
                )
                    .chain()
                    .in_set(OrdoSet)
                    // Between the two, and both edges matter: `relayout` sets the padding a
                    // stepper's arrows then trim, and `paint_buttons` reads the `Selected` a
                    // tab puts on.
                    .after(widgets::relayout)
                    .before(widgets::paint_buttons),
            )
            // After the button paint, which it overwrites for tabs: that pass writes one
            // color all the way round and a tab is the one widget that wants an edge of its
            // own.
            .add_systems(
                Update,
                tabs::paint_tabs
                    .in_set(OrdoSet)
                    .after(widgets::paint_buttons),
            )
            .add_observer(tabs::open_tab);

        if let Some(path) = &self.theme_path {
            let handle = app.world().resource::<AssetServer>().load(path.clone());
            app.insert_resource(ThemeHandle(handle));
        }
    }
}
