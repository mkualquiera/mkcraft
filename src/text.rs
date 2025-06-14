use std::error::Error;

use gl33::GlFns;
use ndarray::Array2;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{
    mesh::{Mesh, MeshEnvelope, MeshParams},
    toki::Logograph,
    utils::*,
};

#[derive(Copy, Clone, Debug)]
enum Vowel {
    A,
    E,
    I,
    O,
    U,
}

impl Vowel {
    fn column(&self) -> u8 {
        match self {
            Vowel::A => 0,
            Vowel::E => 1,
            Vowel::I => 2,
            Vowel::O => 3,
            Vowel::U => 4,
        }
    }
    fn from_char(c: char) -> Option<Self> {
        match c {
            'A' => Some(Vowel::A),
            'E' => Some(Vowel::E),
            'I' => Some(Vowel::I),
            'O' => Some(Vowel::O),
            'U' => Some(Vowel::U),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Consonant {
    K,
    L,
    M,
    N,
    P,
    S,
    T,
    J,
    W,
}

impl Consonant {
    fn row(&self) -> u8 {
        match self {
            Consonant::K => 1,
            Consonant::L => 2,
            Consonant::M => 3,
            Consonant::N => 4,
            Consonant::P => 5,
            Consonant::S => 6,
            Consonant::T => 7,
            Consonant::J => 8,
            Consonant::W => 9,
        }
    }
    fn from_char(c: char) -> Option<Self> {
        match c {
            'K' => Some(Consonant::K),
            'L' => Some(Consonant::L),
            'M' => Some(Consonant::M),
            'N' => Some(Consonant::N),
            'P' => Some(Consonant::P),
            'S' => Some(Consonant::S),
            'T' => Some(Consonant::T),
            'J' => Some(Consonant::J),
            'W' => Some(Consonant::W),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Glyph {
    Pair(Consonant, Vowel),
    Single(Vowel),
    N,
    Blank,
    OpenQuote,
    CloseQuote,
    OpenAngleQuote,
    CloseAngleQuote,
    Period,
    Colon,
    Comma,
    Ellipsis,
    Logograph(Logograph),
}

impl Glyph {
    fn material_id(&self) -> [u8; 2] {
        match self {
            Glyph::Pair(c, v) => [v.column(), c.row()],
            Glyph::Single(v) => [v.column(), 0],
            Glyph::N => [5, 0],
            Glyph::Blank => [5, 1],
            Glyph::OpenQuote => [5, 2],
            Glyph::CloseQuote => [5, 3],
            Glyph::OpenAngleQuote => [5, 4],
            Glyph::CloseAngleQuote => [5, 5],
            Glyph::Period => [5, 6],
            Glyph::Colon => [5, 7],
            Glyph::Comma => [5, 8],
            Glyph::Ellipsis => [5, 9],
            Glyph::Logograph(logograph) => logograph.material_id(),
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        if s.len() == 3 {
            if s == "..." {
                return Some(Glyph::Ellipsis);
            }
        } else if s.len() == 1 {
            if s == "N" {
                return Some(Glyph::N);
            }
            if s == "[" {
                return Some(Glyph::OpenQuote);
            }
            if s == "]" {
                return Some(Glyph::CloseQuote);
            }
            if s == "." {
                return Some(Glyph::Period);
            }
            if s == ":" {
                return Some(Glyph::Colon);
            }
            if s == "," {
                return Some(Glyph::Comma);
            }
            if let Some(vowel) = Vowel::from_char(s.chars().next().unwrap()) {
                return Some(Glyph::Single(vowel));
            }
        } else if s.len() == 2 {
            if s == "<<" {
                return Some(Glyph::OpenAngleQuote);
            }
            if s == ">>" {
                return Some(Glyph::CloseAngleQuote);
            }
            if let (Some(c), Some(v)) = (
                Consonant::from_char(s.chars().next().unwrap()),
                Vowel::from_char(s.chars().nth(1).unwrap()),
            ) {
                return Some(Glyph::Pair(c, v));
            }
        }
        None
    }
    fn parse_latin(s: &str) -> Result<Vec<Self>, Box<dyn Error>> {
        // Parses a latin word into a vector of Glyphs.
        let mut glyphs = Vec::new();
        let mut buffer = s.to_string();

        let mut logographs_sorted = Logograph::options();
        logographs_sorted.sort_by_key(|logograph| logograph.len());
        logographs_sorted.reverse();

        'outer: while !buffer.is_empty() {
            for logograph in &logographs_sorted {
                if buffer.starts_with(logograph) {
                    // If the buffer starts with a logograph, we parse it
                    glyphs.push(Glyph::Logograph(
                        Logograph::from_str(logograph)
                            .expect("Unable to parse logograph"),
                    ));
                    buffer = buffer[logograph.len()..].to_string(); // Remove the logograph from the buffer
                    continue 'outer;
                }
            }
            if buffer.len() > 2 && Glyph::from_str(&buffer[0..3]).is_some() {
                // If the first three characters form a valid glyph
                glyphs.push(Glyph::from_str(&buffer[0..3]).unwrap());
                buffer = buffer[3..].to_string(); // Remove the first three characters
            } else if buffer.len() > 1 && Glyph::from_str(&buffer[0..2]).is_some() {
                // If the first two characters form a valid glyph
                glyphs.push(Glyph::from_str(&buffer[0..2]).unwrap());
                buffer = buffer[2..].to_string(); // Remove the first two characters
            } else if let Some(glyph) = Glyph::from_str(&buffer[0..1]) {
                glyphs.push(glyph);
                buffer = buffer[1..].to_string(); // Remove the first character
            } else {
                // Parsing error
                return Err(format!("Invalid glyph in word: {}", s).into());
            }
        }
        if glyphs.is_empty() {
            return Err(format!("No valid glyphs found in word: {}", s).into());
        }

        return Ok(glyphs);
    }
}

#[derive(Clone, Copy, Debug)]
struct RenderableGlyph {
    glyph: Glyph,
    background_color: [f32; 4],
    foreground_color: [f32; 4],
}

pub enum MeshOrigin {
    TL, // Top Left
    TC, // Top Center
    TR, // Top Right
    BL, // Bottom Left
    BC, // Bottom Center
    BR, // Bottom Right
    CC,
}

impl RenderableGlyph {
    fn space() -> Self {
        RenderableGlyph {
            glyph: Glyph::Blank,
            background_color: [0.0, 0.0, 0.0, 0.0],
            foreground_color: [0.0, 0.0, 0.0, 0.0],
        }
    }
    fn tessellate_glyph(
        &self,
        x: f32,
        y: f32,
        z: f32,
        vertices: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
        colors: &mut Vec<[f32; 4]>,
        materials: &mut Vec<[i32; 2]>,
        lights: &mut Vec<[f32; 4]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        let vertex_count = vertices.len() as u32;
        vertices.push([
            BACK_BOTTOM_LEFT_X + x,
            BACK_BOTTOM_LEFT_Y + y,
            BACK_BOTTOM_LEFT_Z + z - 1.0,
        ]);
        vertices.push([
            BACK_BOTTOM_RIGHT_X + x,
            BACK_BOTTOM_RIGHT_Y + y,
            BACK_BOTTOM_RIGHT_Z + z - 1.0,
        ]);
        vertices.push([
            BACK_TOP_RIGHT_X + x,
            BACK_TOP_RIGHT_Y + y,
            BACK_TOP_RIGHT_Z + z - 1.0,
        ]);
        vertices.push([
            BACK_TOP_LEFT_X + x,
            BACK_TOP_LEFT_Y + y,
            BACK_TOP_LEFT_Z + z - 1.0,
        ]);
        uvs.push([0.0, 1.0]);
        uvs.push([1.0, 1.0]);
        uvs.push([1.0, 0.0]);
        uvs.push([0.0, 0.0]);
        indices.push(vertex_count);
        indices.push(vertex_count + 1);
        indices.push(vertex_count + 2);
        indices.push(vertex_count + 2);
        indices.push(vertex_count + 3);
        indices.push(vertex_count);
        for i in 0..4 {
            colors.push(self.foreground_color);
            materials.push(self.glyph.material_id().map(|x| x as i32));
            lights.push(self.background_color);
        }
    }
    fn tessellate_glyphs(
        glyphs: Array2<RenderableGlyph>,
        origin: &MeshOrigin,
    ) -> MeshEnvelope {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut material_ids = Vec::new();
        let mut colors = Vec::new();
        let mut light = Vec::new();
        let mut uvs = Vec::new();

        let chars_per_line = glyphs.shape()[1] as f32;
        let lines = glyphs.shape()[0] as f32;

        let (ox, oy) = match origin {
            MeshOrigin::TR => (0.0, 0.0),
            MeshOrigin::BL => (lines - 1.0, chars_per_line - 1.0),
            MeshOrigin::BC => (lines / 2.0, chars_per_line / 2.0),
            MeshOrigin::BR => (lines, 0.0),
            MeshOrigin::TL => (0.0, chars_per_line),
            MeshOrigin::TC => (lines / 2.0, chars_per_line / 2.0),
            MeshOrigin::CC => (lines / 2.0, chars_per_line / 2.0),
        };

        for char in 0..glyphs.shape()[1] {
            for line in 0..glyphs.shape()[0] {
                let glyph = &glyphs[[line, char]];
                glyph.tessellate_glyph(
                    -(line as f32) + ox - 1.0,
                    -(char as f32) + oy - 1.0,
                    0.0,
                    &mut vertices,
                    &mut indices,
                    &mut colors,
                    &mut material_ids,
                    &mut light,
                    &mut uvs,
                );
            }
        }

        MeshEnvelope::new(MeshParams {
            vertices,
            indices: Some(indices),
            uvs: Some(uvs),
            material_ids: Some(material_ids),
            colors: Some(colors),
            light: Some(light),
        })
    }
}

#[derive(Debug)]
struct Word {
    syllables: Vec<RenderableGlyph>,
    with_space: bool,
}

impl Word {
    fn len(&self) -> usize {
        self.syllables.len()
    }
}

#[derive(Debug)]
enum TextPiece {
    Word(Word),
    LineBreak,
}

#[derive(Debug)]
pub struct Text {
    words: Vec<TextPiece>,
}

enum TypesettingElement {
    WordElement(Word),
    SpaceElement,
}

impl TypesettingElement {
    fn get_width(&self) -> usize {
        match self {
            TypesettingElement::WordElement(word) => word.len(),
            TypesettingElement::SpaceElement => 1, // Space is considered as one unit width
        }
    }
}

#[derive(Clone, Copy)]
pub enum Alignment {
    Top, // Would be left in a left-to-right language but here it's top
    Center,
    Bottom, // Would be right in a left-to-right language but here it's bottom
    Justify,
}

struct TypesettedLine {
    elements: Vec<TypesettingElement>,
}

impl TypesettedLine {
    fn from_text(
        mut text: Text,
        alignment: Alignment,
        max_width: usize,
    ) -> (Option<Self>, Text) {
        if text.words.is_empty() {
            return (None, text);
        }
        if text.words.len() == 1 {
            if let TextPiece::Word(word) = &text.words[0] {
                if word.len() > max_width {
                    // If the word is longer than max width, we can't typeset it
                    return (None, Text { words: Vec::new() });
                }
            }
        }
        let mut elements = Vec::new();
        let mut current_line_width: usize = 0;
        // General algorithm is, we start by assuming left alignment,
        // Then we just add spaces depending on the alignment.

        // so for now we grab words until we fill max_width
        while current_line_width < max_width && !text.words.is_empty() {
            let with_space = match text.words.remove(0) {
                TextPiece::Word(word) => {
                    let word_width = word.len();
                    if current_line_width + word_width > max_width {
                        // If adding this word exceeds max width, we stop here
                        text.words.insert(0, TextPiece::Word(word));
                        break;
                    }
                    let has_space = word.with_space;
                    elements.push(TypesettingElement::WordElement(word));
                    current_line_width += word_width;
                    has_space
                }
                TextPiece::LineBreak => {
                    // If we encounter a line break, we stop the current line
                    break;
                }
            };
            // If we still have text, add a space
            if !text.words.is_empty() && current_line_width < max_width && with_space {
                elements.push(TypesettingElement::SpaceElement);
                current_line_width += 1; // Space is considered as one unit width
            }
        }

        // Ensure we never have a space at the end of the line
        if let Some(TypesettingElement::SpaceElement) = elements.last() {
            elements.pop();
            current_line_width -= 1; // Remove the space from the current line width
        }

        // Now we have the content of the line, we need to handle alignment
        match alignment {
            Alignment::Top => {
                while current_line_width < max_width {
                    elements.push(TypesettingElement::SpaceElement);
                    current_line_width += 1;
                }
            }
            Alignment::Bottom => {
                let spaces_needed = max_width - current_line_width;
                for _ in 0..spaces_needed {
                    elements.insert(0, TypesettingElement::SpaceElement);
                }
            }
            Alignment::Center => {
                let spaces_needed = max_width - current_line_width;
                let left_spaces = spaces_needed / 2;
                let right_spaces = spaces_needed - left_spaces;

                for _ in 0..left_spaces {
                    elements.insert(0, TypesettingElement::SpaceElement);
                }
                for _ in 0..right_spaces {
                    elements.push(TypesettingElement::SpaceElement);
                }
            }
            Alignment::Justify => {
                let spaces_needed = max_width - current_line_width;
                // Convert random spaces into double spaces until we fill the line
                for _ in 0..spaces_needed {
                    // Find a space
                    let space_positions = elements
                        .iter()
                        .enumerate()
                        .filter_map(|(i, el)| {
                            if matches!(el, TypesettingElement::SpaceElement) {
                                Some(i)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    if space_positions.is_empty() {
                        break; // Can't double any spaces
                    }
                    // use line lenght as seed
                    let mut rng = StdRng::seed_from_u64(current_line_width as u64);
                    // Pick a random space position
                    let random_index = rng.random_range(0..space_positions.len());
                    let space_pos = space_positions[random_index];
                    // Double the space
                    elements.insert(space_pos + 1, TypesettingElement::SpaceElement);
                    // Update current line width
                    current_line_width += 1;
                }
            }
        }

        (Some(TypesettedLine { elements }), text)
    }
    fn into_glyphs(self) -> Vec<RenderableGlyph> {
        self.elements
            .into_iter()
            .flat_map(|el| match el {
                TypesettingElement::WordElement(word) => word.syllables,
                TypesettingElement::SpaceElement => vec![RenderableGlyph::space()],
            })
            .collect()
    }
}

fn split_spaces<'a>(text: &'a str) -> Vec<(Option<&'a str>, Option<&'a str>)> {
    let mut result = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut last = 0;

    while let Some((index, ch)) = chars.next() {
        if ch == ' ' {
            // Check if this is a double space
            let is_double_space = chars
                .peek()
                .map(|(_, next_ch)| *next_ch == ' ')
                .unwrap_or(false);

            // Add content before the space(s) if any
            let content = if last != index {
                Some(&text[last..index])
            } else {
                None
            };

            if is_double_space {
                // Consume the second space
                chars.next();
                result.push((Some("  "), content));
                last = index + 2; // Skip both spaces
            } else {
                result.push((Some(" "), content));
                last = index + 1; // Skip single space
            }
        }
    }

    // Add any remaining content
    if last < text.len() {
        let content = &text[last..];
        result.push((None, Some(content)));
    }

    result
}

impl Text {
    pub fn from_spec(spec: &str) -> Result<Self, Box<dyn Error>> {
        let mut current_foreground = [1.0, 1.0, 1.0, 1.0];
        let mut current_background = [0.3, 0.3, 0.3, 1.0];
        let lines = spec.lines().collect::<Vec<_>>();
        let mut pieces = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            for (separator, word) in split_spaces(line) {
                let word = word.expect("No word found in line");
                if word.is_empty() {
                    println!("word is empty, skipping");
                    continue;
                }
                if word.starts_with("f:") || word.starts_with("F:") {
                    // Change foreground color
                    let color_str = &word["f:".len()..];
                    let color = parse_color(color_str)?;
                    current_foreground = color;
                    continue;
                } else if word.starts_with("b:") || word.starts_with("B:") {
                    // Change background color
                    let color_str = &word["b:".len()..];
                    let color = parse_color(color_str)?;
                    current_background = color;
                    continue;
                } else if word.starts_with("reset") || word.starts_with("RESET") {
                    // Reset colors to default
                    current_foreground = [1.0, 1.0, 1.0, 1.0];
                    current_background = [0.3, 0.3, 0.3, 1.0];
                    continue;
                }
                let word_glyphs = Glyph::parse_latin(word)?;
                let mut syllables = Vec::new();
                for glyph in word_glyphs {
                    syllables.push(RenderableGlyph {
                        glyph,
                        background_color: current_background,
                        foreground_color: current_foreground,
                    });
                }
                if syllables.is_empty() {
                    return Err(
                        format!("No valid glyphs found in word: {}", word).into()
                    );
                }
                // If separator is some and it is \s\s then we add a space
                let is_space = if let Some(separator) = separator {
                    if separator == "  " {
                        // Add a space after the word
                        true
                    } else {
                        // No space after the word
                        false
                    }
                } else {
                    // No separator, no space
                    false
                };
                pieces.push(TextPiece::Word(Word {
                    syllables,
                    with_space: is_space,
                }));
            }

            // Add a line break after each line except the last one
            if i < lines.len() - 1 {
                pieces.push(TextPiece::LineBreak);
            }
        }
        Ok(Text { words: pieces })
    }
}

fn parse_color(color_str: &str) -> Result<[f32; 4], Box<dyn Error>> {
    // Parse as html
    if color_str.starts_with('#') {
        let hex = &color_str[1..];
        if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            let a = u8::from_str_radix(&hex[6..8], 16)?;
            Ok([
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ])
        } else {
            Err("Invalid hex color format".into())
        }
    } else {
        Err("Only hex color format is supported".into())
    }
}

pub struct RenderableText {
    mesh: MeshEnvelope,
}

pub struct TextOptions {
    pub alignment: Alignment,
    pub origin: MeshOrigin,
    pub max_width: usize,
}

impl TextOptions {
    pub fn new(max_width: usize) -> Self {
        TextOptions {
            alignment: Alignment::Top,
            origin: MeshOrigin::TR,
            max_width,
        }
    }
    pub fn set_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
    pub fn set_max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }
    pub fn set_origin(mut self, origin: MeshOrigin) -> Self {
        self.origin = origin;
        self
    }
    pub fn render_spec(&self, spec: &str) -> Result<RenderableText, Box<dyn Error>> {
        let text = Text::from_spec(spec)?;
        let mut remaining_text = text;
        let mut lines = Vec::new();

        while let (Some(line), rest) =
            TypesettedLine::from_text(remaining_text, self.alignment, self.max_width)
        {
            remaining_text = rest;
            lines.push(line);
        }

        if lines.is_empty() {
            return Err("No valid lines to render".into());
        }

        let num_lines = lines.len();

        // reverse lines because we render from bottom to top

        let glyphs = lines
            .into_iter()
            .flat_map(|line| line.into_glyphs())
            .collect::<Vec<_>>();

        let glyph_array =
            Array2::from_shape_vec((num_lines, self.max_width), glyphs)
                .map_err(|e| format!("Failed to create glyph array: {}", e))?;

        let mesh = RenderableGlyph::tessellate_glyphs(glyph_array, &self.origin);
        Ok(RenderableText { mesh })
    }
}

impl RenderableText {
    pub fn get_mesh(&mut self, gl: &GlFns) -> &Mesh {
        self.mesh.get_mesh(gl)
    }
}

pub fn into_syllabic(text: &str) -> String {
    // All lowercase to uppercase
    let text = text.to_uppercase();
    // Replace spaces with double spaces
    let text = text.replace(' ', "  ");
    return text;
}
