use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

/// The kind of value an MJML attribute accepts. Used for completion and hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrType {
    /// Free-form text.
    Text,
    /// A CSS color (hex, rgb, or named).
    Color,
    /// A length such as `10px`, `100%`, or `2em`.
    Measure,
    /// One of a fixed set of values.
    Enum(&'static [&'static str]),
    /// A link.
    Url,
    /// A boolean-like flag.
    Boolean,
}

impl AttrType {
    /// A short, human-readable label for the value type (used by completion and hover).
    pub fn label(self) -> String {
        match self {
            Self::Text => "text".to_string(),
            Self::Color => "color".to_string(),
            Self::Measure => "measure".to_string(),
            Self::Url => "url".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::Enum(values) => values.join(" | "),
        }
    }
}

/// Describes a single attribute of an MJML component.
#[derive(Debug, Clone, Copy)]
pub struct AttrSpec {
    pub name: &'static str,
    pub ty: AttrType,
    pub default: Option<&'static str>,
    pub required: bool,
    pub doc: &'static str,
}

/// Describes an MJML component (tag): its documentation, nesting rules, and attributes.
#[derive(Debug, Clone, Copy)]
pub struct ComponentSpec {
    pub name: &'static str,
    pub doc: &'static str,
    pub docs_url: &'static str,
    /// Whether the tag has a closing tag (`true`) or is self-closing (`false`).
    #[cfg_attr(not(test), expect(dead_code))]
    pub ending_tag: bool,
    /// Tags this component may appear directly inside. Empty for the root (`mjml`).
    pub allowed_parents: &'static [&'static str],
    pub attributes: &'static [AttrSpec],
}

/// The hand-authored MJML component model, seeded from the official MJML documentation.
///
/// This is the single source of truth for known tags, nesting rules, required
/// attributes, and the documentation surfaced by completion and hover.
const COMPONENTS: &[ComponentSpec] = &[
    // ----- Root -----
    ComponentSpec {
        name: "mjml",
        doc: "Root element of an MJML document.",
        docs_url: "https://documentation.mjml.io/",
        ending_tag: true,
        allowed_parents: &[],
        attributes: &[
            AttrSpec {
                name: "lang",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Document language, e.g. en.",
            },
            AttrSpec {
                name: "dir",
                ty: AttrType::Enum(&["auto", "ltr", "rtl"]),
                default: Some("auto"),
                required: false,
                doc: "Base text direction of the document.",
            },
        ],
    },
    // ----- Head -----
    ComponentSpec {
        name: "mj-head",
        doc: "Container for document-level settings such as styles, fonts, and default attributes.",
        docs_url: "https://documentation.mjml.io/#mj-head",
        ending_tag: true,
        allowed_parents: &["mjml"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-title",
        doc: "Sets the document title, used as the inbox preview line in some clients.",
        docs_url: "https://documentation.mjml.io/#mj-title",
        ending_tag: true,
        allowed_parents: &["mj-head"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-preview",
        doc: "Sets the preview text shown next to the subject line in the inbox.",
        docs_url: "https://documentation.mjml.io/#mj-preview",
        ending_tag: true,
        allowed_parents: &["mj-head"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-style",
        doc: "Adds CSS styles to the document head.",
        docs_url: "https://documentation.mjml.io/#mj-style",
        ending_tag: true,
        allowed_parents: &["mj-head"],
        attributes: &[AttrSpec {
            name: "inline",
            ty: AttrType::Boolean,
            default: None,
            required: false,
            doc: "Set to inline to inline the styles onto elements.",
        }],
    },
    ComponentSpec {
        name: "mj-font",
        doc: "Imports a web font for use in the document.",
        docs_url: "https://documentation.mjml.io/#mj-font",
        ending_tag: false,
        allowed_parents: &["mj-head"],
        attributes: &[
            AttrSpec {
                name: "name",
                ty: AttrType::Text,
                default: None,
                required: true,
                doc: "Font name referenced in font-family.",
            },
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: true,
                doc: "Link to the font stylesheet.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-breakpoint",
        doc: "Sets the width at which the layout switches to its mobile version.",
        docs_url: "https://documentation.mjml.io/#mj-breakpoint",
        ending_tag: false,
        allowed_parents: &["mj-head"],
        attributes: &[AttrSpec {
            name: "width",
            ty: AttrType::Measure,
            default: None,
            required: true,
            doc: "Breakpoint width, e.g. 480px.",
        }],
    },
    ComponentSpec {
        name: "mj-attributes",
        doc: "Sets default attributes for components across the whole document.",
        docs_url: "https://documentation.mjml.io/#mj-attributes",
        ending_tag: true,
        allowed_parents: &["mj-head"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-html-attributes",
        doc: "Adds custom HTML attributes to elements matched by a selector.",
        docs_url: "https://documentation.mjml.io/#mj-html-attributes",
        ending_tag: true,
        allowed_parents: &["mj-head"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-all",
        doc: "Sets default attributes applied to every component.",
        docs_url: "https://documentation.mjml.io/#mj-attributes",
        ending_tag: false,
        allowed_parents: &["mj-attributes"],
        attributes: &[],
    },
    ComponentSpec {
        name: "mj-class",
        doc: "Defines a named set of attributes, applied to elements via the mj-class attribute.",
        docs_url: "https://documentation.mjml.io/#mj-attributes",
        ending_tag: false,
        allowed_parents: &["mj-attributes"],
        attributes: &[AttrSpec {
            name: "name",
            ty: AttrType::Text,
            default: None,
            required: true,
            doc: "Class name referenced via the mj-class attribute.",
        }],
    },
    ComponentSpec {
        name: "mj-selector",
        doc: "Targets elements by CSS selector for mj-html-attributes.",
        docs_url: "https://documentation.mjml.io/#mj-html-attributes",
        ending_tag: true,
        allowed_parents: &["mj-html-attributes"],
        attributes: &[AttrSpec {
            name: "path",
            ty: AttrType::Text,
            default: None,
            required: false,
            doc: "CSS selector identifying the elements to target.",
        }],
    },
    ComponentSpec {
        name: "mj-html-attribute",
        doc: "Sets a single custom HTML attribute on the selected elements.",
        docs_url: "https://documentation.mjml.io/#mj-html-attributes",
        ending_tag: true,
        allowed_parents: &["mj-selector"],
        attributes: &[AttrSpec {
            name: "name",
            ty: AttrType::Text,
            default: None,
            required: false,
            doc: "Name of the HTML attribute to set.",
        }],
    },
    // ----- Body and layout -----
    ComponentSpec {
        name: "mj-body",
        doc: "Container for the email body content.",
        docs_url: "https://documentation.mjml.io/#mj-body",
        ending_tag: true,
        allowed_parents: &["mjml"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the whole body.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: Some("600px"),
                required: false,
                doc: "Email content width.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-wrapper",
        doc: "Wraps several sections so they can share a background.",
        docs_url: "https://documentation.mjml.io/#mj-wrapper",
        ending_tag: true,
        allowed_parents: &["mj-body"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color.",
            },
            AttrSpec {
                name: "background-url",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Background image link.",
            },
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Corner radius.",
            },
            AttrSpec {
                name: "full-width",
                ty: AttrType::Enum(&["full-width"]),
                default: None,
                required: false,
                doc: "Set to full-width to span the full viewport width.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("20px 0"),
                required: false,
                doc: "Padding around the wrapper.",
            },
            AttrSpec {
                name: "text-align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of inner content.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-section",
        doc: "A row-level layout container. Sections hold columns.",
        docs_url: "https://documentation.mjml.io/#mj-section",
        ending_tag: true,
        allowed_parents: &["mj-body", "mj-wrapper"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Section background color.",
            },
            AttrSpec {
                name: "background-url",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Background image link.",
            },
            AttrSpec {
                name: "background-repeat",
                ty: AttrType::Enum(&["repeat", "no-repeat"]),
                default: Some("repeat"),
                required: false,
                doc: "How the background image repeats.",
            },
            AttrSpec {
                name: "background-size",
                ty: AttrType::Text,
                default: Some("auto"),
                required: false,
                doc: "Background image size.",
            },
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Corner radius.",
            },
            AttrSpec {
                name: "direction",
                ty: AttrType::Enum(&["ltr", "rtl"]),
                default: Some("ltr"),
                required: false,
                doc: "Order in which columns are placed.",
            },
            AttrSpec {
                name: "full-width",
                ty: AttrType::Enum(&["full-width"]),
                default: None,
                required: false,
                doc: "Set to full-width to span the full viewport width.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("20px 0"),
                required: false,
                doc: "Padding around the section.",
            },
            AttrSpec {
                name: "text-align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of inner content.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-group",
        doc: "Groups columns so they stay side by side on mobile instead of stacking.",
        docs_url: "https://documentation.mjml.io/#mj-group",
        ending_tag: true,
        allowed_parents: &["mj-section"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color.",
            },
            AttrSpec {
                name: "direction",
                ty: AttrType::Enum(&["ltr", "rtl"]),
                default: Some("ltr"),
                required: false,
                doc: "Order in which columns are placed.",
            },
            AttrSpec {
                name: "vertical-align",
                ty: AttrType::Enum(&["top", "middle", "bottom"]),
                default: Some("top"),
                required: false,
                doc: "Vertical alignment within the group.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Group width.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-column",
        doc: "A vertical container inside a section. Columns hold content components.",
        docs_url: "https://documentation.mjml.io/#mj-column",
        ending_tag: true,
        allowed_parents: &["mj-section", "mj-group"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Column background color.",
            },
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Corner radius.",
            },
            AttrSpec {
                name: "inner-border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Border applied to the inner table.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Padding around the column content.",
            },
            AttrSpec {
                name: "vertical-align",
                ty: AttrType::Enum(&["top", "middle", "bottom"]),
                default: Some("top"),
                required: false,
                doc: "Vertical alignment within the row.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Column width.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-hero",
        doc: "A hero block with a background image and overlaid content.",
        docs_url: "https://documentation.mjml.io/#mj-hero",
        ending_tag: true,
        allowed_parents: &["mj-body"],
        attributes: &[
            AttrSpec {
                name: "mode",
                ty: AttrType::Enum(&["fluid-height", "fixed-height"]),
                default: Some("fixed-height"),
                required: false,
                doc: "Whether the hero height is fixed or fluid.",
            },
            AttrSpec {
                name: "height",
                ty: AttrType::Measure,
                default: Some("0px"),
                required: false,
                doc: "Hero height when using fixed-height mode.",
            },
            AttrSpec {
                name: "background-url",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Background image link.",
            },
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color shown behind the image.",
            },
            AttrSpec {
                name: "background-position",
                ty: AttrType::Text,
                default: Some("center center"),
                required: false,
                doc: "Background image position.",
            },
            AttrSpec {
                name: "vertical-align",
                ty: AttrType::Enum(&["top", "middle", "bottom"]),
                default: Some("top"),
                required: false,
                doc: "Vertical alignment of the content.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("0px"),
                required: false,
                doc: "Padding around the content.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    // ----- Content -----
    ComponentSpec {
        name: "mj-text",
        doc: "Displays a block of text or inline HTML.",
        docs_url: "https://documentation.mjml.io/#mj-text",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: Some("#000000"),
                required: false,
                doc: "Text color.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: Some("Ubuntu, Helvetica, Arial, sans-serif"),
                required: false,
                doc: "Font family.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: Some("13px"),
                required: false,
                doc: "Font size.",
            },
            AttrSpec {
                name: "font-style",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font style, e.g. italic.",
            },
            AttrSpec {
                name: "font-weight",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font weight, e.g. bold or 400.",
            },
            AttrSpec {
                name: "line-height",
                ty: AttrType::Measure,
                default: Some("1"),
                required: false,
                doc: "Line height.",
            },
            AttrSpec {
                name: "letter-spacing",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Spacing between letters.",
            },
            AttrSpec {
                name: "text-decoration",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Text decoration, e.g. underline.",
            },
            AttrSpec {
                name: "text-transform",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Text transform, e.g. uppercase.",
            },
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "right", "center", "justify"]),
                default: Some("left"),
                required: false,
                doc: "Horizontal alignment.",
            },
            AttrSpec {
                name: "height",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Fixed height of the text block.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the text.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the text.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-button",
        doc: "A clickable call-to-action button.",
        docs_url: "https://documentation.mjml.io/#mj-button",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Link the button points to.",
            },
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: Some("#414141"),
                required: false,
                doc: "Button background color.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: Some("#ffffff"),
                required: false,
                doc: "Button text color.",
            },
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: Some("3px"),
                required: false,
                doc: "Corner radius.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font family.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: Some("13px"),
                required: false,
                doc: "Font size.",
            },
            AttrSpec {
                name: "font-weight",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font weight.",
            },
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of the button.",
            },
            AttrSpec {
                name: "inner-padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding inside the button.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the button.",
            },
            AttrSpec {
                name: "target",
                ty: AttrType::Text,
                default: Some("_blank"),
                required: false,
                doc: "Link target.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Button width.",
            },
            AttrSpec {
                name: "height",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Button height.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the button.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-image",
        doc: "Displays a responsive image.",
        docs_url: "https://documentation.mjml.io/#mj-image",
        ending_tag: false,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "src",
                ty: AttrType::Url,
                default: None,
                required: true,
                doc: "Image source link.",
            },
            AttrSpec {
                name: "alt",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Alternative text.",
            },
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Link the image points to.",
            },
            AttrSpec {
                name: "title",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Image title attribute.",
            },
            AttrSpec {
                name: "target",
                ty: AttrType::Text,
                default: Some("_blank"),
                required: false,
                doc: "Link target.",
            },
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of the image.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Image width.",
            },
            AttrSpec {
                name: "height",
                ty: AttrType::Measure,
                default: Some("auto"),
                required: false,
                doc: "Image height.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Corner radius.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the image.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the image.",
            },
            AttrSpec {
                name: "fluid-on-mobile",
                ty: AttrType::Enum(&["true", "false"]),
                default: None,
                required: false,
                doc: "When true, the image spans the full width on mobile.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-divider",
        doc: "A horizontal divider line.",
        docs_url: "https://documentation.mjml.io/#mj-divider",
        ending_tag: false,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "border-color",
                ty: AttrType::Color,
                default: Some("#000000"),
                required: false,
                doc: "Divider color.",
            },
            AttrSpec {
                name: "border-style",
                ty: AttrType::Enum(&["solid", "dashed", "dotted"]),
                default: Some("solid"),
                required: false,
                doc: "Divider line style.",
            },
            AttrSpec {
                name: "border-width",
                ty: AttrType::Measure,
                default: Some("4px"),
                required: false,
                doc: "Divider thickness.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: Some("100%"),
                required: false,
                doc: "Divider width.",
            },
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the divider.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the divider.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-spacer",
        doc: "Adds vertical space.",
        docs_url: "https://documentation.mjml.io/#mj-spacer",
        ending_tag: false,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "height",
                ty: AttrType::Measure,
                default: Some("20px"),
                required: false,
                doc: "Amount of vertical space.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Padding around the spacer.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the spacer.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-table",
        doc: "Displays tabular data.",
        docs_url: "https://documentation.mjml.io/#mj-table",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("left"),
                required: false,
                doc: "Horizontal alignment of the table.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: Some("#000000"),
                required: false,
                doc: "Text color.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font family.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: Some("13px"),
                required: false,
                doc: "Font size.",
            },
            AttrSpec {
                name: "line-height",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Line height.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the table.",
            },
            AttrSpec {
                name: "table-layout",
                ty: AttrType::Enum(&["auto", "fixed", "initial", "inherit"]),
                default: Some("auto"),
                required: false,
                doc: "CSS table-layout value.",
            },
            AttrSpec {
                name: "width",
                ty: AttrType::Measure,
                default: Some("100%"),
                required: false,
                doc: "Table width.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the cell containing the table.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-raw",
        doc: "Outputs raw HTML that MJML passes through unprocessed.",
        docs_url: "https://documentation.mjml.io/#mj-raw",
        ending_tag: true,
        allowed_parents: &["mj-head", "mj-column", "mj-hero"],
        attributes: &[AttrSpec {
            name: "position",
            ty: AttrType::Enum(&["file-start"]),
            default: None,
            required: false,
            doc: "Set to file-start to output the content before the doctype.",
        }],
    },
    // ----- Interactive -----
    ComponentSpec {
        name: "mj-social",
        doc: "Displays a row of social network icons.",
        docs_url: "https://documentation.mjml.io/#mj-social",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of the icons.",
            },
            AttrSpec {
                name: "mode",
                ty: AttrType::Enum(&["horizontal", "vertical"]),
                default: Some("horizontal"),
                required: false,
                doc: "Whether icons are laid out in a row or column.",
            },
            AttrSpec {
                name: "icon-size",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Icon size.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Label text color.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Label font size.",
            },
            AttrSpec {
                name: "border-radius",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Icon corner radius.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("10px 25px"),
                required: false,
                doc: "Padding around the icons.",
            },
            AttrSpec {
                name: "container-background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the containing cell.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-social-element",
        doc: "A single social network entry inside mj-social.",
        docs_url: "https://documentation.mjml.io/#mj-social-element",
        ending_tag: true,
        allowed_parents: &["mj-social"],
        attributes: &[
            AttrSpec {
                name: "name",
                ty: AttrType::Text,
                default: None,
                required: true,
                doc: "Social network name, e.g. facebook or twitter.",
            },
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Link the icon points to.",
            },
            AttrSpec {
                name: "src",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Custom icon image link.",
            },
            AttrSpec {
                name: "alt",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Alternative text for the icon.",
            },
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Icon background color.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-accordion",
        doc: "A collapsible accordion block.",
        docs_url: "https://documentation.mjml.io/#mj-accordion",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font family.",
            },
            AttrSpec {
                name: "icon-align",
                ty: AttrType::Enum(&["top", "middle", "bottom"]),
                default: Some("middle"),
                required: false,
                doc: "Vertical alignment of the toggle icon.",
            },
            AttrSpec {
                name: "icon-position",
                ty: AttrType::Enum(&["left", "right"]),
                default: Some("right"),
                required: false,
                doc: "Side the toggle icon appears on.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Padding around the accordion.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-accordion-element",
        doc: "A single expandable item within an accordion.",
        docs_url: "https://documentation.mjml.io/#mj-accordion",
        ending_tag: true,
        allowed_parents: &["mj-accordion"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Background color of the item.",
            },
            AttrSpec {
                name: "border",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "CSS border shorthand.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font family.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-accordion-title",
        doc: "The clickable title of an accordion element.",
        docs_url: "https://documentation.mjml.io/#mj-accordion",
        ending_tag: true,
        allowed_parents: &["mj-accordion-element"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Title background color.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Title text color.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Title font size.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Padding around the title.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-accordion-text",
        doc: "The expandable content of an accordion element.",
        docs_url: "https://documentation.mjml.io/#mj-accordion",
        ending_tag: true,
        allowed_parents: &["mj-accordion-element"],
        attributes: &[
            AttrSpec {
                name: "background-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Content background color.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Content text color.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Content font size.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Padding around the content.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-carousel",
        doc: "An interactive image carousel.",
        docs_url: "https://documentation.mjml.io/#mj-carousel",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of the carousel.",
            },
            AttrSpec {
                name: "icon-width",
                ty: AttrType::Measure,
                default: None,
                required: false,
                doc: "Width of the navigation icons.",
            },
            AttrSpec {
                name: "left-icon",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Custom previous-arrow image link.",
            },
            AttrSpec {
                name: "right-icon",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Custom next-arrow image link.",
            },
            AttrSpec {
                name: "thumbnails",
                ty: AttrType::Enum(&["visible", "hidden"]),
                default: Some("visible"),
                required: false,
                doc: "Whether thumbnails are shown.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-carousel-image",
        doc: "A single image within a carousel.",
        docs_url: "https://documentation.mjml.io/#mj-carousel",
        ending_tag: false,
        allowed_parents: &["mj-carousel"],
        attributes: &[
            AttrSpec {
                name: "src",
                ty: AttrType::Url,
                default: None,
                required: true,
                doc: "Image source link.",
            },
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Link the image points to.",
            },
            AttrSpec {
                name: "alt",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Alternative text.",
            },
            AttrSpec {
                name: "title",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Image title attribute.",
            },
            AttrSpec {
                name: "target",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Link target.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-navbar",
        doc: "A navigation bar that collapses to a hamburger menu on mobile.",
        docs_url: "https://documentation.mjml.io/#mj-navbar",
        ending_tag: true,
        allowed_parents: &["mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "align",
                ty: AttrType::Enum(&["left", "center", "right"]),
                default: Some("center"),
                required: false,
                doc: "Horizontal alignment of the links.",
            },
            AttrSpec {
                name: "base-url",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Base link prepended to each navbar link.",
            },
            AttrSpec {
                name: "hamburger",
                ty: AttrType::Enum(&["hamburger"]),
                default: None,
                required: false,
                doc: "Set to hamburger to enable the mobile menu.",
            },
            AttrSpec {
                name: "ico-color",
                ty: AttrType::Color,
                default: None,
                required: false,
                doc: "Hamburger icon color.",
            },
            AttrSpec {
                name: "css-class",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Class name(s) applied to the rendered element.",
            },
        ],
    },
    ComponentSpec {
        name: "mj-navbar-link",
        doc: "A single link inside an mj-navbar.",
        docs_url: "https://documentation.mjml.io/#mj-navbar",
        ending_tag: true,
        allowed_parents: &["mj-navbar"],
        attributes: &[
            AttrSpec {
                name: "href",
                ty: AttrType::Url,
                default: None,
                required: false,
                doc: "Link target.",
            },
            AttrSpec {
                name: "color",
                ty: AttrType::Color,
                default: Some("#000000"),
                required: false,
                doc: "Link text color.",
            },
            AttrSpec {
                name: "font-family",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Font family.",
            },
            AttrSpec {
                name: "font-size",
                ty: AttrType::Measure,
                default: Some("13px"),
                required: false,
                doc: "Font size.",
            },
            AttrSpec {
                name: "padding",
                ty: AttrType::Measure,
                default: Some("15px 10px"),
                required: false,
                doc: "Padding around the link.",
            },
            AttrSpec {
                name: "target",
                ty: AttrType::Text,
                default: Some("_blank"),
                required: false,
                doc: "Link target attribute.",
            },
        ],
    },
    // ----- Includes -----
    ComponentSpec {
        name: "mj-include",
        doc: "Includes the content of another MJML, HTML, or CSS file.",
        docs_url: "https://documentation.mjml.io/#mj-include",
        ending_tag: false,
        allowed_parents: &["mj-head", "mj-body", "mj-column", "mj-hero"],
        attributes: &[
            AttrSpec {
                name: "path",
                ty: AttrType::Text,
                default: None,
                required: false,
                doc: "Path to the file to include.",
            },
            AttrSpec {
                name: "type",
                ty: AttrType::Enum(&["mjml", "html", "css"]),
                default: Some("mjml"),
                required: false,
                doc: "Kind of file being included.",
            },
        ],
    },
];

/// Registry of all MJML components, keyed by tag name.
pub static REGISTRY: LazyLock<HashMap<&'static str, &'static ComponentSpec>> =
    LazyLock::new(|| COMPONENTS.iter().map(|c| (c.name, c)).collect());

/// All known MJML tag names.
pub static KNOWN_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| COMPONENTS.iter().map(|c| c.name).collect());

/// Returns the component spec for `tag`, if it is a known MJML element.
pub fn component(tag: &str) -> Option<&'static ComponentSpec> {
    REGISTRY.get(tag).copied()
}

/// Returns allowed parent tag names for a given MJML tag, or None if it is the
/// root element (`mjml`) or not a known MJML tag.
pub fn allowed_parents(tag: &str) -> Option<&'static [&'static str]> {
    match component(tag) {
        Some(c) if !c.allowed_parents.is_empty() => Some(c.allowed_parents),
        _ => None,
    }
}

/// Returns the names of components that may appear directly inside `parent`.
pub fn allowed_children(parent: &str) -> Vec<&'static str> {
    COMPONENTS
        .iter()
        .filter(|c| c.allowed_parents.contains(&parent))
        .map(|c| c.name)
        .collect()
}

/// Returns required attribute names for a given MJML tag. Empty if the tag has
/// no required attributes or is unknown.
pub fn required_attributes(tag: &str) -> Vec<&'static str> {
    let Some(c) = component(tag) else {
        return Vec::new();
    };
    c.attributes
        .iter()
        .filter(|a| a.required)
        .map(|a| a.name)
        .collect()
}

/// Returns the spec for attribute `attr` on `tag`, if known.
pub fn attribute(tag: &str, attr: &str) -> Option<&'static AttrSpec> {
    component(tag).and_then(|c| c.attributes.iter().find(|a| a.name == attr))
}

/// If `tag` is an unknown mj-* tag close to a known one (edit distance <= 2),
/// returns the closest known tag. Returns None if exact match or no close match.
pub fn suggest_tag(tag: &str) -> Option<&'static str> {
    if KNOWN_TAGS.contains(tag) {
        return None; // exact match, no suggestion needed
    }
    let mut best: Option<(&str, usize)> = None;
    for &known in &*KNOWN_TAGS {
        let d = edit_distance(tag, known);
        if d <= 2 && (best.is_none() || d < best.unwrap().1) {
            best = Some((known, d));
        }
    }
    best.map(|(tag, _)| tag)
}

/// Simple Levenshtein distance.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact set of tags that existed before the registry refactor.
    /// Guards against accidental additions or removals during the migration.
    const ORIGINAL_TAGS: &[&str] = &[
        "mjml",
        "mj-head",
        "mj-body",
        "mj-title",
        "mj-preview",
        "mj-style",
        "mj-font",
        "mj-breakpoint",
        "mj-attributes",
        "mj-html-attributes",
        "mj-all",
        "mj-class",
        "mj-selector",
        "mj-html-attribute",
        "mj-section",
        "mj-wrapper",
        "mj-hero",
        "mj-group",
        "mj-column",
        "mj-text",
        "mj-image",
        "mj-button",
        "mj-divider",
        "mj-spacer",
        "mj-social",
        "mj-social-element",
        "mj-accordion",
        "mj-accordion-element",
        "mj-accordion-title",
        "mj-accordion-text",
        "mj-carousel",
        "mj-carousel-image",
        "mj-navbar",
        "mj-navbar-link",
        "mj-table",
        "mj-raw",
        "mj-include",
    ];

    #[test]
    fn test_registry_covers_exactly_the_original_tags() {
        assert_eq!(KNOWN_TAGS.len(), ORIGINAL_TAGS.len());
        for tag in ORIGINAL_TAGS {
            assert!(KNOWN_TAGS.contains(tag), "registry is missing {tag}");
        }
    }

    #[test]
    fn test_known_tags_contains_all_mjml_elements() {
        assert!(KNOWN_TAGS.contains("mjml"));
        assert!(KNOWN_TAGS.contains("mj-body"));
        assert!(KNOWN_TAGS.contains("mj-head"));
        assert!(KNOWN_TAGS.contains("mj-section"));
        assert!(KNOWN_TAGS.contains("mj-column"));
        assert!(KNOWN_TAGS.contains("mj-text"));
        assert!(KNOWN_TAGS.contains("mj-image"));
        assert!(KNOWN_TAGS.contains("mj-button"));
        assert!(KNOWN_TAGS.contains("mj-social"));
        assert!(KNOWN_TAGS.contains("mj-social-element"));
        assert!(KNOWN_TAGS.contains("mj-accordion"));
        assert!(KNOWN_TAGS.contains("mj-accordion-element"));
        assert!(KNOWN_TAGS.contains("mj-accordion-title"));
        assert!(KNOWN_TAGS.contains("mj-accordion-text"));
        assert!(KNOWN_TAGS.contains("mj-carousel"));
        assert!(KNOWN_TAGS.contains("mj-carousel-image"));
        assert!(KNOWN_TAGS.contains("mj-navbar"));
        assert!(KNOWN_TAGS.contains("mj-navbar-link"));
        assert!(!KNOWN_TAGS.contains("mj-nonexistent"));
    }

    #[test]
    fn test_component_lookup() {
        let img = component("mj-image").expect("mj-image should be known");
        assert_eq!(img.name, "mj-image");
        assert!(!img.ending_tag, "mj-image is self-closing");
        assert!(img.docs_url.contains("mj-image"));
        assert!(!img.doc.is_empty());
        assert!(component("div").is_none());
    }

    #[test]
    fn test_allowed_parents_mj_text() {
        let parents = allowed_parents("mj-text");
        assert!(parents.is_some());
        let parents = parents.unwrap();
        assert!(parents.contains(&"mj-column"));
        assert!(parents.contains(&"mj-hero"));
        assert!(!parents.contains(&"mj-body"));
    }

    #[test]
    fn test_allowed_parents_mj_head() {
        let parents = allowed_parents("mj-head");
        assert!(parents.is_some());
        assert!(parents.unwrap().contains(&"mjml"));
    }

    #[test]
    fn test_allowed_parents_root_is_none() {
        // mjml is the root and has no parent.
        assert!(allowed_parents("mjml").is_none());
    }

    #[test]
    fn test_allowed_parents_unknown_tag() {
        assert!(allowed_parents("div").is_none());
    }

    #[test]
    fn test_allowed_children() {
        let body_children = allowed_children("mj-body");
        assert!(body_children.contains(&"mj-section"));
        assert!(body_children.contains(&"mj-wrapper"));
        assert!(body_children.contains(&"mj-hero"));
        assert!(!body_children.contains(&"mj-head"));

        let section_children = allowed_children("mj-section");
        assert!(section_children.contains(&"mj-column"));
        assert!(section_children.contains(&"mj-group"));
    }

    #[test]
    fn test_required_attrs_mj_image() {
        let attrs = required_attributes("mj-image");
        assert!(attrs.contains(&"src"));
    }

    #[test]
    fn test_required_attrs_mj_text() {
        // mj-text has no required attributes
        assert!(required_attributes("mj-text").is_empty());
    }

    #[test]
    fn test_required_attrs_match_original_rules() {
        // Pin the full set of required attributes to preserve diagnostics behavior.
        assert_eq!(required_attributes("mj-image"), vec!["src"]);
        assert_eq!(required_attributes("mj-carousel-image"), vec!["src"]);
        assert_eq!(required_attributes("mj-font"), vec!["name", "href"]);
        assert_eq!(required_attributes("mj-breakpoint"), vec!["width"]);
        assert_eq!(required_attributes("mj-class"), vec!["name"]);
        assert_eq!(required_attributes("mj-social-element"), vec!["name"]);
        // Tags that previously had no required attributes must stay empty.
        assert!(required_attributes("mj-section").is_empty());
        assert!(required_attributes("mj-include").is_empty());
        assert!(required_attributes("mj-selector").is_empty());
        assert!(required_attributes("mjml").is_empty());
    }

    #[test]
    fn test_attribute_lookup() {
        let src = attribute("mj-image", "src").expect("mj-image has src");
        assert!(src.required);
        assert_eq!(src.ty, AttrType::Url);
        assert!(src.default.is_none());
        assert!(!src.doc.is_empty());
        assert!(attribute("mj-image", "not-an-attr").is_none());

        let align = attribute("mj-text", "align").expect("mj-text has align");
        assert_eq!(
            align.ty,
            AttrType::Enum(&["left", "right", "center", "justify"])
        );
    }

    #[test]
    fn test_suggest_tag_typo() {
        assert_eq!(suggest_tag("mj-seciton"), Some("mj-section"));
        assert_eq!(suggest_tag("mj-buton"), Some("mj-button"));
        assert_eq!(suggest_tag("mj-section"), None); // exact match = no suggestion needed
        assert_eq!(suggest_tag("mj-xyzabc"), None); // too far
    }
}
