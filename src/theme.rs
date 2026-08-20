//! Roles, metrics, and the file that fills them.
//!
//! Ordo does not own color. A game's interface should be cut from the same
//! cloth as its world — Divus Factus tints its panels from the very ramps its
//! villagers' clothes are dyed from — and a kit that shipped its own palette
//! would force every game to keep two sets of colors in step forever. So Ordo
//! owns the *vocabulary* (what a `CardBorder` is for, how wide a label column
//! should be) and the game supplies the pigment.
//!
//! Nothing here reads a color at spawn time. A node is tagged with the *role*
//! it plays — [`Fill`], [`Edge`], [`Ink`], [`Face`], [`TextSize`] — and a repaint
//! pass fills it in whenever the theme changes. That indirection is the whole
//! reason the theme file can be edited with the game running.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, ColorStop, InterpolationColorSpace, LinearGradient};
use serde::Deserialize;
use std::collections::HashMap;

/// What a color is *for*. Roles, not names: a panel border stays a panel
/// border when the game turns from gold to verdigris, and every call site that
/// asked for one comes along without being touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Panel background. Near opaque by convention — a bright landscape
    /// bleeding through is what makes long dim text unreadable.
    PanelBg,
    /// Title-bar fill, a shade off the panel so chrome reads as its own part.
    TitleBg,
    /// The warm well: detail panes that should read as a second material.
    CardBg,
    CardBorder,
    PanelBorder,
    /// The dimmer a modal sits on. Distinct from [`Role::PanelBg`], which is
    /// near opaque on purpose — a scrim that hid the world would defeat itself.
    Scrim,
    /// Primary text.
    Ink,
    /// Secondary text — labels, hints, anything the eye should find second but
    /// still *find*.
    InkDim,
    /// Emphasis. Titles, and the occasional word that matters.
    Accent,
    ButtonIdle,
    ButtonHover,
    ButtonPressed,
}

/// What a typeface is *for*.
///
/// Divus Factus already keeps exactly these three — Cinzel for display,
/// CinzelDecorative-Bold beside it, EB Garamond for reading — so this is its
/// vocabulary rather than an invented one. `DisplayBold` is a separate role
/// and not a weight because it is a separate family, which is how display
/// pairings usually go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontRole {
    Display,
    DisplayBold,
    Body,
}

/// A typeface the theme resolved, ready to drop onto a `TextFont`.
#[derive(Debug, Clone)]
pub struct FontFace {
    pub source: FontSource,
    pub weight: FontWeight,
    pub style: FontStyle,
}

/// A tunable number. Same argument as [`Role`]: the call site asks for the
/// label column, not for 112 pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    TitleSize,
    /// A herald's voice: the proclamation card's title, and nothing else.
    /// Every other heading in an interface labels something the eye is
    /// already looking at; this one has to carry a room, so it is a size
    /// of its own rather than a multiple of the ordinary title.
    HeraldSize,
    /// The line beneath a herald's title. Its own metric, because it is the
    /// only text in the kit whose job is to sit under something enormous:
    /// falling through to [`Metric::SmallSize`] put a 12px line under a 38px
    /// title, which read as a caption that had wandered in rather than as the
    /// second half of a sentence.
    HeraldLineSize,
    BodySize,
    SmallSize,
    /// Inner padding of a panel.
    Pad,
    /// Gap between rows inside a panel.
    Gap,
    /// Distance from the screen edge to an anchored panel.
    Margin,
    /// Width of the label column in a stat row. One width everywhere is what
    /// makes stacked rows read as a table instead of a list of sentences.
    LabelWidth,
    RowHeight,
    Border,
    Radius,
    /// Distance from a radial menu's hub to its wedges.
    RadialRadius,
    /// Radius around a radial hub inside which nothing is selected yet, so
    /// opening a menu does not commit to whichever way the hand drifted.
    RadialDeadZone,
}

/// The resolved theme. Systems read this; nothing writes it but
/// [`apply_theme_asset`] and whatever the game does on top.
#[derive(Resource, Debug, Clone)]
pub struct Theme {
    colors: HashMap<Role, Color>,
    metrics: HashMap<Metric, f32>,
    fonts: HashMap<FontRole, FontFace>,
}

impl Theme {
    pub fn color(&self, role: Role) -> Color {
        self.colors.get(&role).copied().unwrap_or(Color::WHITE)
    }

    /// The typeface for a role, if the game named one.
    ///
    /// `None` is the ordinary case for a game that has no opinion — Flat Earth
    /// Simulator ships no fonts at all — and it means "leave `TextFont` alone",
    /// which lands on Bevy's embedded default rather than on nothing.
    pub fn font(&self, role: FontRole) -> Option<&FontFace> {
        self.fonts.get(&role)
    }

    pub fn set_font(&mut self, role: FontRole, face: FontFace) {
        self.fonts.insert(role, face);
    }

    pub fn metric(&self, metric: Metric) -> f32 {
        self.metrics.get(&metric).copied().unwrap_or_default()
    }

    /// Metric as a [`Val`], which is how nine uses in ten want it.
    pub fn px(&self, metric: Metric) -> Val {
        px(self.metric(metric))
    }

    pub fn set_color(&mut self, role: Role, color: Color) {
        self.colors.insert(role, color);
    }

    pub fn set_metric(&mut self, metric: Metric, value: f32) {
        self.metrics.insert(metric, value);
    }
}

impl Default for Theme {
    /// A complete, coherent cool-slate default, so a game that never writes a
    /// theme file still looks deliberate rather than unstyled.
    fn default() -> Self {
        use Metric::*;
        use Role::*;
        Self {
            colors: HashMap::from([
                (PanelBg, Color::srgba(0.06, 0.08, 0.11, 0.96)),
                (TitleBg, Color::srgba(0.09, 0.11, 0.15, 0.98)),
                (CardBg, Color::srgb(0.08, 0.10, 0.13)),
                (CardBorder, Color::srgb(0.24, 0.29, 0.36)),
                (PanelBorder, Color::srgb(0.30, 0.36, 0.44)),
                (Scrim, Color::srgba(0.02, 0.03, 0.05, 0.72)),
                (Ink, Color::srgb(0.90, 0.92, 0.95)),
                (InkDim, Color::srgb(0.62, 0.67, 0.74)),
                (Accent, Color::srgb(0.55, 0.78, 0.95)),
                (ButtonIdle, Color::srgb(0.11, 0.14, 0.18)),
                (ButtonHover, Color::srgb(0.19, 0.25, 0.32)),
                (ButtonPressed, Color::srgb(0.30, 0.40, 0.50)),
            ]),
            metrics: HashMap::from([
                (TitleSize, 15.0),
                (HeraldSize, 38.0),
                (HeraldLineSize, 20.0),
                (BodySize, 13.0),
                (SmallSize, 12.0),
                (Pad, 12.0),
                (Gap, 5.0),
                (Margin, 10.0),
                (LabelWidth, 112.0),
                (RowHeight, 22.0),
                (Border, 1.0),
                (Radius, 5.0),
                (RadialRadius, 84.0),
                (RadialDeadZone, 26.0),
            ]),
            // Empty on purpose. Ordo ships no typeface for the same reason it
            // ships no palette.
            fonts: HashMap::new(),
        }
    }
}

/// Color ramps the game lends to Ordo, by name.
///
/// A ramp is registered as the game's *own* sampling function rather than as a
/// list of stops, so `{ ramp = "cloth_gold", shade = 0.85 }` in the theme file
/// resolves through exactly the code the rest of the game uses. There is no
/// second interpolation to drift out of step with the first.
///
/// ```ignore
/// ramps.register("cloth_gold", |t| palette::shade(&palette::CLOTH_GOLD, t));
/// ```
#[derive(Resource, Default)]
pub struct Ramps {
    map: HashMap<String, Box<dyn Fn(f32) -> Color + Send + Sync>>,
}

impl Ramps {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        sample: impl Fn(f32) -> Color + Send + Sync + 'static,
    ) {
        self.map.insert(name.into(), Box::new(sample));
    }

    pub fn sample(&self, name: &str, shade: f32) -> Option<Color> {
        self.map.get(name).map(|f| f(shade))
    }
}

// ---------------------------------------------------------------------------
// Tags. A node says what it is; the repaint pass says what that looks like.
// ---------------------------------------------------------------------------

/// This node's `BackgroundColor` follows a role.
#[derive(Component, Debug, Clone, Copy)]
pub struct Fill(pub Role);

/// This node's `BorderColor` follows a role.
#[derive(Component, Debug, Clone, Copy)]
pub struct Edge(pub Role);

/// A vertical two-role gradient fill — the kit's material depth. A panel
/// wearing a sheen reads as a surface with light falling on it instead of
/// a flat rectangle, which is most of the distance between "programmer
/// UI" and "someone crafted this". Painted from the theme like
/// [`Fill`]; the alphas ride along so chrome can stay translucent.
#[derive(Component, Debug, Clone, Copy)]
pub struct Sheen {
    pub top: Role,
    pub top_alpha: f32,
    pub bottom: Role,
    pub bottom_alpha: f32,
}

impl Sheen {
    pub fn new(top: Role, top_alpha: f32, bottom: Role, bottom_alpha: f32) -> Self {
        Sheen {
            top,
            top_alpha,
            bottom,
            bottom_alpha,
        }
    }
}

/// This text's `TextColor` follows a role.
#[derive(Component, Debug, Clone, Copy)]
pub struct Ink(pub Role);

/// This text's size follows a metric.
#[derive(Component, Debug, Clone, Copy)]
pub struct TextSize(pub Metric);

/// This text's typeface follows a font role.
///
/// Absent, or naming a role the game never filled in, leaves `TextFont` alone —
/// which is Bevy's embedded default, not nothing.
#[derive(Component, Debug, Clone, Copy)]
pub struct Face(pub FontRole);

/// Scales whatever alpha this node's role already carries.
///
/// Animation goes through here rather than writing colors directly, so the
/// repaint pass stays the only thing that ever sets a color. Two writers
/// racing over one `BackgroundColor` is how a fading toast ends up flickering
/// back to full opacity every time the theme file is touched.
#[derive(Component, Debug, Clone, Copy)]
pub struct Opacity(pub f32);

impl Default for Opacity {
    fn default() -> Self {
        Self(1.0)
    }
}

/// A role's color, scaled by whatever [`Opacity`] the node carries.
///
/// Shared with `widgets::paint_buttons`, which is the only painter outside
/// [`repaint`] — a button's color depends on the pointer, which the repaint
/// pass knows nothing about. Both go through here so neither can forget the
/// opacity and leave half a widget unfaded.
pub(crate) fn tinted(theme: &Theme, role: Role, opacity: Option<&Opacity>) -> Color {
    let color = theme.color(role);
    match opacity {
        Some(Opacity(scale)) => color.with_alpha(color.alpha() * scale),
        None => color,
    }
}

// ---------------------------------------------------------------------------
// The asset
// ---------------------------------------------------------------------------

/// A color in the theme file: either stated outright, or drawn off one of the
/// game's ramps.
///
/// ```toml
/// panel_bg = { srgb = [0.045, 0.05, 0.062], alpha = 0.985 }
/// accent   = { ramp = "cloth_gold", shade = 0.85 }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorSpec {
    Ramp {
        ramp: String,
        shade: f32,
        #[serde(default = "opaque")]
        alpha: f32,
    },
    Srgb {
        srgb: [f32; 3],
        #[serde(default = "opaque")]
        alpha: f32,
    },
}

fn opaque() -> f32 {
    1.0
}

/// A typeface in the theme file. Three ways to name one, in order of how
/// specific they are:
///
/// ```toml
/// display = { path = "fonts/Cinzel.ttf" }   # an asset the game ships
/// body    = { family = "EB Garamond" }      # by name, from the font database
/// mono    = { generic = "monospace" }       # let the platform decide
/// ```
///
/// A struct with optional fields rather than an untagged enum: serde's untagged
/// errors collapse to "did not match any variant", which is a miserable thing
/// to read while editing a live file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FontSpec {
    pub path: Option<String>,
    pub family: Option<String>,
    pub generic: Option<String>,
    /// 1–1000. Only variable fonts honor anything but the weight they were cut
    /// at; a separate bold file wants its own role.
    pub weight: Option<u16>,
    pub italic: bool,
}

impl FontSpec {
    fn resolve(&self, assets: &AssetServer) -> Option<FontFace> {
        let source = match (&self.path, &self.family, &self.generic) {
            (Some(path), _, _) => FontSource::Handle(assets.load(path.clone())),
            (_, Some(family), _) => FontSource::Family(family.as_str().into()),
            (_, _, Some(generic)) => generic_source(generic)?,
            _ => return None,
        };
        Some(FontFace {
            source,
            weight: self.weight.map(FontWeight).unwrap_or(FontWeight::NORMAL),
            style: if self.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
        })
    }
}

/// The CSS generic families, spelled the way CSS spells them.
fn generic_source(name: &str) -> Option<FontSource> {
    Some(match name {
        "serif" => FontSource::Serif,
        "sans-serif" => FontSource::SansSerif,
        "monospace" => FontSource::Monospace,
        "cursive" => FontSource::Cursive,
        "fantasy" => FontSource::Fantasy,
        "system-ui" => FontSource::SystemUi,
        "ui-serif" => FontSource::UiSerif,
        "ui-sans-serif" => FontSource::UiSansSerif,
        "ui-monospace" => FontSource::UiMonospace,
        "ui-rounded" => FontSource::UiRounded,
        "emoji" => FontSource::Emoji,
        "math" => FontSource::Math,
        _ => {
            warn!("theme names unknown generic font {name:?}");
            return None;
        }
    })
}

impl ColorSpec {
    fn resolve(&self, ramps: &Ramps) -> Option<Color> {
        match self {
            ColorSpec::Srgb { srgb, alpha } => {
                Some(Color::srgba(srgb[0], srgb[1], srgb[2], *alpha))
            }
            ColorSpec::Ramp { ramp, shade, alpha } => {
                ramps.sample(ramp, *shade).map(|c| c.with_alpha(*alpha))
            }
        }
    }
}

/// Every field optional, so a game's file states only what it disagrees with
/// and inherits the rest. Unknown keys are an error rather than a shrug — a
/// typo in a hot-reloaded file that silently does nothing is the worst way to
/// spend twenty minutes.
#[derive(Asset, TypePath, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ThemeAsset {
    pub color: ColorTable,
    pub metric: MetricTable,
    pub font: FontTable,
}

/// Both tables are the same shape — a named optional per variant, plus a way to
/// walk the ones that were actually stated — so they are written once.
macro_rules! table {
    ($name:ident, $key:ty, $value:ty, $($field:ident => $variant:ident),+ $(,)?) => {
        #[derive(Debug, Default, Deserialize)]
        #[serde(deny_unknown_fields, default)]
        #[allow(missing_docs)]
        pub struct $name {
            $(pub $field: Option<$value>,)+
        }

        impl $name {
            fn entries(&self) -> Vec<($key, &$value)> {
                let mut out = Vec::new();
                $(if let Some(v) = &self.$field { out.push((<$key>::$variant, v)); })+
                out
            }
        }
    };
}

table!(
    ColorTable, Role, ColorSpec,
    panel_bg => PanelBg,
    title_bg => TitleBg,
    card_bg => CardBg,
    card_border => CardBorder,
    panel_border => PanelBorder,
    scrim => Scrim,
    ink => Ink,
    ink_dim => InkDim,
    accent => Accent,
    button_idle => ButtonIdle,
    button_hover => ButtonHover,
    button_pressed => ButtonPressed,
);

table!(
    FontTable, FontRole, FontSpec,
    display => Display,
    display_bold => DisplayBold,
    body => Body,
);

table!(
    MetricTable, Metric, f32,
    title_size => TitleSize,
    herald_size => HeraldSize,
    herald_line_size => HeraldLineSize,
    body_size => BodySize,
    small_size => SmallSize,
    pad => Pad,
    gap => Gap,
    margin => Margin,
    label_width => LabelWidth,
    row_height => RowHeight,
    border => Border,
    radius => Radius,
    radial_radius => RadialRadius,
    radial_dead_zone => RadialDeadZone,
);

/// Keeps the loaded theme file alive and findable.
#[derive(Resource)]
pub struct ThemeHandle(pub Handle<ThemeAsset>);

#[derive(Default, TypePath)]
pub(crate) struct ThemeLoader;

/// Hand-rolled rather than derived: two variants and one `Display` is less
/// code than the attribute that would write it, and one fewer dependency.
#[derive(Debug)]
pub enum ThemeLoadError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read theme: {e}"),
            Self::Parse(e) => write!(f, "could not parse theme: {e}"),
        }
    }
}

impl std::error::Error for ThemeLoadError {}

impl From<std::io::Error> for ThemeLoadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ThemeLoadError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

impl AssetLoader for ThemeLoader {
    type Asset = ThemeAsset;
    type Settings = ();
    type Error = ThemeLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _ctx: &mut LoadContext<'_>,
    ) -> Result<ThemeAsset, ThemeLoadError> {
        let mut text = String::new();
        reader.read_to_string(&mut text).await?;
        Ok(toml::from_str(&text)?)
    }

    fn extensions(&self) -> &[&str] {
        &["ordo.toml"]
    }
}

/// Folds the loaded file into [`Theme`].
///
/// Watches `Assets<ThemeAsset>` for change rather than reading asset events:
/// coarser, but it cannot miss an edit, and it costs one flag check a frame.
pub(crate) fn apply_theme_asset(
    assets: Res<Assets<ThemeAsset>>,
    handle: Option<Res<ThemeHandle>>,
    ramps: Res<Ramps>,
    server: Res<AssetServer>,
    mut theme: ResMut<Theme>,
) {
    if !(assets.is_changed() || ramps.is_changed()) {
        return;
    }
    let Some(handle) = handle else { return };
    let Some(asset) = assets.get(&handle.0) else {
        return;
    };

    for (role, spec) in asset.color.entries() {
        match spec.resolve(&ramps) {
            Some(color) => theme.set_color(role, color),
            None => {
                if let ColorSpec::Ramp { ramp, .. } = spec {
                    warn!("theme names ramp {ramp:?}, which no game has registered");
                }
            }
        }
    }
    for (metric, value) in asset.metric.entries() {
        theme.set_metric(metric, *value);
    }
    for (role, spec) in asset.font.entries() {
        // `AssetServer::load` hands back the same handle for the same path, so
        // re-resolving on every edit of the file costs nothing.
        match spec.resolve(&server) {
            Some(face) => theme.set_font(role, face),
            None => warn!("theme names {role:?} but gives it no path, family or generic"),
        }
    }
}

// ---------------------------------------------------------------------------
// Repaint
// ---------------------------------------------------------------------------

/// Fills in every tagged node when the theme moves, and when a node is first
/// tagged. This is the pass that makes an edit to the theme file show up in a
/// running game.
pub(crate) fn repaint(
    theme: Res<Theme>,
    mut fills: Query<(&Fill, Option<&Opacity>, &mut BackgroundColor)>,
    mut sheens: Query<(&Sheen, Option<&Opacity>, &mut BackgroundGradient)>,
    mut edges: Query<(&Edge, Option<&Opacity>, &mut BorderColor)>,
    mut inks: Query<(&Ink, Option<&Opacity>, &mut TextColor)>,
    // One query, because `TextSize` and `Face` both write `TextFont` and two
    // systems reaching for it is a conflict Bevy will refuse.
    mut typography: Query<
        (Option<&TextSize>, Option<&Face>, &mut TextFont),
        Or<(With<TextSize>, With<Face>)>,
    >,
    fresh: Query<
        (),
        Or<(
            Added<Fill>,
            Added<Sheen>,
            Added<Edge>,
            Added<Ink>,
            Added<TextSize>,
            Added<Face>,
            Changed<Opacity>,
        )>,
    >,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }
    for (fill, opacity, mut background) in &mut fills {
        *background = BackgroundColor(tinted(&theme, fill.0, opacity));
    }
    for (sheen, opacity, mut gradient) in &mut sheens {
        let mut top = tinted(&theme, sheen.top, opacity);
        top.set_alpha(top.alpha() * sheen.top_alpha);
        let mut bottom = tinted(&theme, sheen.bottom, opacity);
        bottom.set_alpha(bottom.alpha() * sheen.bottom_alpha);
        *gradient = BackgroundGradient::from(LinearGradient {
            color_space: InterpolationColorSpace::default(),
            angle: LinearGradient::TO_BOTTOM,
            stops: vec![
                ColorStop::new(top, Val::Percent(0.0)),
                ColorStop::new(bottom, Val::Percent(100.0)),
            ],
        });
    }
    for (edge, opacity, mut border) in &mut edges {
        *border = BorderColor::all(tinted(&theme, edge.0, opacity));
    }
    for (ink, opacity, mut color) in &mut inks {
        *color = TextColor(tinted(&theme, ink.0, opacity));
    }
    for (size, face, mut font) in &mut typography {
        if let Some(size) = size {
            font.font_size = FontSize::Px(theme.metric(size.0));
        }
        if let Some(Face(role)) = face
            && let Some(resolved) = theme.font(*role)
        {
            font.font = resolved.source.clone();
            font.weight = resolved.weight;
            font.style = resolved.style;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file that ships with the kit is also the worked example in the
    /// README, so a schema that drifts away from it is a bug in both.
    #[test]
    fn the_shipped_theme_parses() {
        let text = include_str!("../assets/theme.ordo.toml");
        let asset: ThemeAsset = toml::from_str(text).expect("shipped theme should parse");
        assert_eq!(asset.metric.label_width, Some(112.0));
        assert!(matches!(
            asset.color.accent,
            Some(ColorSpec::Ramp { shade, .. }) if shade == 0.85
        ));
        assert!(matches!(asset.color.panel_bg, Some(ColorSpec::Srgb { .. })));
        assert_eq!(
            asset.font.display_bold.as_ref().and_then(|f| f.weight),
            Some(700)
        );
    }

    #[test]
    fn a_font_can_be_named_three_ways_and_a_nameless_one_resolves_to_nothing() {
        let parsed: ThemeAsset = toml::from_str(
            "[font]\n\
             display = { path = \"fonts/Cinzel.ttf\" }\n\
             display_bold = { family = \"EB Garamond\", italic = true }\n\
             body = { generic = \"monospace\" }\n",
        )
        .expect("all three forms should parse");
        assert!(parsed.font.display.as_ref().unwrap().path.is_some());
        assert!(parsed.font.display_bold.as_ref().unwrap().italic);
        assert_eq!(
            parsed.font.body.as_ref().unwrap().generic.as_deref(),
            Some("monospace")
        );

        // A spec that names nothing cannot resolve, and must not be guessed at.
        let empty: ThemeAsset =
            toml::from_str("[font]\nbody = { weight = 700 }\n").expect("parses");
        assert!(empty.font.body.as_ref().unwrap().path.is_none());
    }

    #[test]
    fn an_unknown_key_is_an_error_rather_than_a_shrug() {
        let typo = "[color]\npanel_bgg = { srgb = [0.0, 0.0, 0.0] }\n";
        assert!(toml::from_str::<ThemeAsset>(typo).is_err());
    }

    #[test]
    fn a_file_may_state_only_what_it_disagrees_with() {
        let partial = "[metric]\npad = 20.0\n";
        let asset: ThemeAsset = toml::from_str(partial).expect("partial theme should parse");
        assert_eq!(asset.metric.pad, Some(20.0));
        assert_eq!(asset.metric.gap, None);
        assert!(asset.color.accent.is_none());
    }

    #[test]
    fn a_ramp_resolves_through_the_games_own_sampler() {
        let mut ramps = Ramps::default();
        // Deliberately not a plain lerp: the point of handing over a closure is
        // that whatever the game does here is what the theme gets.
        ramps.register("gold", |t| Color::srgb(t * t, 0.0, 0.0));
        let spec = ColorSpec::Ramp {
            ramp: "gold".into(),
            shade: 0.5,
            alpha: 0.25,
        };
        let color = spec
            .resolve(&ramps)
            .expect("registered ramp should resolve");
        assert_eq!(color.to_srgba().red, 0.25);
        assert_eq!(color.alpha(), 0.25);
    }

    #[test]
    fn a_ramp_no_game_registered_resolves_to_nothing() {
        let spec = ColorSpec::Ramp {
            ramp: "nobody_lent_this".into(),
            shade: 0.5,
            alpha: 1.0,
        };
        assert!(spec.resolve(&Ramps::default()).is_none());
    }

    /// Defaults have to be complete: a game with no theme file at all should
    /// look deliberate rather than half-painted.
    #[test]
    fn every_role_and_metric_has_a_default() {
        let theme = Theme::default();
        for role in [
            Role::PanelBg,
            Role::TitleBg,
            Role::CardBg,
            Role::CardBorder,
            Role::PanelBorder,
            Role::Scrim,
            Role::Ink,
            Role::InkDim,
            Role::Accent,
            Role::ButtonIdle,
            Role::ButtonHover,
            Role::ButtonPressed,
        ] {
            assert!(theme.colors.contains_key(&role), "no default for {role:?}");
        }
        for metric in [
            Metric::TitleSize,
            Metric::BodySize,
            Metric::HeraldLineSize,
            Metric::SmallSize,
            Metric::Pad,
            Metric::Gap,
            Metric::Margin,
            Metric::LabelWidth,
            Metric::RowHeight,
            Metric::Border,
            Metric::Radius,
            Metric::RadialRadius,
            Metric::RadialDeadZone,
        ] {
            assert!(
                theme.metrics.contains_key(&metric),
                "no default for {metric:?}"
            );
        }
    }
}
