//! A fixed catalog for tests: upstream's suite with each identity spelled out, so the install,
//! detection and UI tests inherited from upstream keep exercising real-shaped apps.
//!
//! Only built for tests (`cfg(test)` here, the `sample-catalog` feature for other crates' tests).
//! Release builds start with an empty catalog and never list these apps.

use crate::catalog::{AppKind, CraftApp};

const ICON: &[u8] = include_bytes!("../tests/fixtures/sample-icon.png");

const fn app(
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    [color, ink_light, ink_dark]: [[u8; 3]; 3],
    repo: &'static str,
    bundle: &'static str,
) -> CraftApp {
    CraftApp {
        id,
        name,
        tagline,
        color,
        ink_light,
        ink_dark,
        repo,
        former_ids: &[],
        former_names: &[],
        kind: AppKind::Official,
        publisher: "ArtCraft",
        msi_publisher: Some("Learning Machines LLC"),
        bundle: Some(bundle),
        former_bundles: &[],
        site: None,
        web_demo: None,
        apple_team: Some("DJ6XS33FX8"),
        upstream: None,
        icon_png: Some(ICON),
    }
}

pub const SUITE: [CraftApp; 14] = [
    app(
        "photocraft",
        "PhotoCraft",
        "Image editing",
        [[0x2f, 0x7b, 0xf5], [0x1a, 0x5b, 0xc4], [0x7f, 0xb0, 0xff]],
        "storytold/photocraft",
        "ai.storyteller.photocraft",
    ),
    app(
        "vectorcraft",
        "VectorCraft",
        "Vector illustration",
        [[0xe8, 0x57, 0x3f], [0xb8, 0x3a, 0x24], [0xff, 0x8a, 0x70]],
        "storytold/vectorcraft",
        "ai.storyteller.vectorcraft",
    ),
    app(
        "filmcraft",
        "FilmCraft",
        "Video editing, color and sound",
        [[0x8b, 0x5c, 0xf6], [0x6a, 0x3f, 0xd6], [0xb6, 0x9c, 0xff]],
        "storytold/filmcraft",
        "ai.storyteller.filmcraft",
    ),
    app(
        "lightcraft",
        "LightCraft",
        "Photo library and raw development",
        [[0xf2, 0xa5, 0x16], [0x8a, 0x58, 0x00], [0xff, 0xc1, 0x4d]],
        "storytold/lightcraft",
        "ai.storyteller.lightcraft",
    ),
    CraftApp {
        former_ids: &["printcraft"],
        former_names: &["PrintCraft"],
        ..app(
            "pdfcraft",
            "PdfCraft",
            "Reading, organizing and protecting PDFs",
            [[0xe8, 0x20, 0x4f], [0xb7, 0x02, 0x28], [0xf2, 0x3a, 0x62]],
            "storytold/pdfcraft",
            "ai.storyteller.pdfcraft",
        )
    },
    app(
        "effectcraft",
        "EffectCraft",
        "Motion graphics and visual effects",
        [[0xe0, 0x36, 0x8f], [0xb0, 0x20, 0x6c], [0xff, 0x7a, 0xb8]],
        "storytold/effectcraft",
        "ai.storyteller.effectcraft",
    ),
    app(
        "designcraft",
        "DesignCraft",
        "Page layout and publishing",
        [[0x7b, 0xb5, 0x1c], [0x45, 0x70, 0x08], [0xa6, 0xe0, 0x4a]],
        "storytold/designcraft",
        "ai.storyteller.designcraft",
    ),
    app(
        "figcraft",
        "FigCraft",
        "Interface design and prototyping",
        [[0x02, 0xda, 0x96], [0x04, 0x7a, 0x56], [0x4f, 0xe8, 0xb6]],
        "storytold/figcraft",
        "ai.storyteller.figcraft",
    ),
    app(
        "wordcraft",
        "WordCraft",
        "Writing and document design",
        [[0x3f, 0x5f, 0xe0], [0x2a, 0x44, 0xb8], [0x8e, 0xa2, 0xff]],
        "storytold/wordcraft",
        "ai.storyteller.wordcraft",
    ),
    app(
        "soundcraft",
        "SoundCraft",
        "Recording, editing and mixing audio",
        [[0x1f, 0xa5, 0xc9], [0x0e, 0x72, 0x90], [0x5f, 0xd0, 0xee]],
        "storytold/soundcraft",
        "ai.storyteller.soundcraft",
    ),
    app(
        "deckcraft",
        "DeckCraft",
        "Presentations and slide shows",
        [[0xf0, 0x7a, 0x1e], [0xa8, 0x4f, 0x0a], [0xff, 0xa0, 0x60]],
        "storytold/deckcraft",
        "ai.storyteller.deckcraft",
    ),
    app(
        "cadcraft",
        "CADCraft",
        "Computer-aided design and drafting",
        [[0x3e, 0xa6, 0xe8], [0x1a, 0x6f, 0xa8], [0x7c, 0xc8, 0xff]],
        "storytold/cadcraft",
        "ai.storyteller.cadcraft",
    ),
    app(
        "gridcraft",
        "GridCraft",
        "Spreadsheets and data",
        [[0x1f, 0x9d, 0x55], [0x12, 0x70, 0x3a], [0x4f, 0xd6, 0x8a]],
        "storytold/gridcraft",
        "ai.storyteller.gridcraft",
    ),
    CraftApp {
        kind: AppKind::Community,
        publisher: "Tangency Labs",
        msi_publisher: Some("SolidCraft contributors"),
        former_bundles: &["ai.storyteller.solidcraft", "ai.storyteller.solvecraft"],
        site: Some("https://solidcraft.io"),
        web_demo: Some("https://solidcraft.io/try"),
        apple_team: None,
        ..app(
            "solidcraft",
            "SolidCraft",
            "Parametric 3D CAD",
            [[0x46, 0xa1, 0xfb], [0x16, 0x53, 0xae], [0x8c, 0xc4, 0xff]],
            "tangency-labs/solidcraft",
            "io.tangencylabs.solidcraft",
        )
    },
];
