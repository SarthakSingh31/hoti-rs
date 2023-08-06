use std::{collections::HashMap, sync::atomic::AtomicUsize};

use glam::UVec2;
use image::{Rgba, RgbaImage};
use taffy::{
    geometry::Point,
    node::MeasureFunc,
    prelude::{Layout, Size},
    style::{AvailableSpace, Position, Style},
    style_helpers::TaffyMaxContent,
};

static IMAGE_HANDLE_IDX: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageHandle(usize);

impl ImageHandle {
    pub fn new() -> Self {
        ImageHandle(IMAGE_HANDLE_IDX.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Debug, Default, Clone)]
struct ImageStore {
    images: HashMap<ImageHandle, RgbaImage>,
    resize_cache: HashMap<ImageHandle, HashMap<UVec2, RgbaImage>>,
}

impl ImageStore {
    pub fn add(&mut self, img: RgbaImage) -> ImageHandle {
        let handle = ImageHandle::new();
        self.images.insert(handle, img);
        handle
    }

    pub fn get(&self, handle: &ImageHandle) -> &RgbaImage {
        &self.images[handle]
    }

    pub fn get_resized(&mut self, handle: &ImageHandle, size: UVec2) -> &RgbaImage {
        let map = self
            .resize_cache
            .entry(*handle)
            .or_insert(HashMap::default());

        map.entry(size).or_insert_with(|| {
            image::imageops::resize(
                &self.images[handle],
                size.x,
                size.y,
                image::imageops::FilterType::Lanczos3,
            )
        })
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    TextCentered {
        text: String,
        font: rusttype::Font<'static>,
        scale: rusttype::Scale,
        line_height: u32,
        color: Rgba<u8>,
    },
    Image(ImageHandle),
    Container(Vec<StyledNode>),
}

#[derive(Debug, Clone)]
pub struct StyledNode {
    pub node: Node,
    pub style: Style,
}

impl StyledNode {
    fn process(
        &self,
        taffy: &mut taffy::Taffy,
        parent: taffy::prelude::Node,
        store: &ImageStore,
    ) -> anyhow::Result<(taffy::prelude::Node, Option<&Vec<StyledNode>>)> {
        let mut children = None;

        let node = match &self.node {
            Node::TextCentered {
                text,
                font,
                scale,
                line_height,
                ..
            } => {
                let text = text.clone();
                let font = font.clone();
                let scale = scale.clone();
                let line_height = line_height.clone();

                let calculate_text_size_for_width = move |width: f32| -> Size<f32> {
                    let words = text.split(' ');
                    let mut lines = Vec::default();

                    let mut current_line = String::default();
                    let mut max_width = 0.0;
                    for word in words {
                        let mut temp_line = current_line.clone();

                        temp_line.push_str(word);

                        let current_width = font
                            .layout(&temp_line, scale, rusttype::point(0.0, 0.0))
                            .last()
                            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
                            .unwrap_or(0.0);

                        if current_width > width && !current_line.is_empty() {
                            lines.push(current_line);
                            current_line = word.to_owned();
                        } else {
                            if current_width > max_width {
                                max_width = current_width;
                            }

                            current_line = temp_line;
                        }

                        current_line.push(' ');
                    }

                    Size {
                        width: max_width,
                        height: (line_height as usize * lines.len()) as f32,
                    }
                };

                taffy.new_leaf_with_measure(
                    self.style.clone(),
                    MeasureFunc::Boxed(Box::new(
                        move |size: Size<Option<f32>>, available: Size<AvailableSpace>| match (
                            size.width,
                            available.width,
                        ) {
                            (None, AvailableSpace::Definite(ava_width)) => {
                                calculate_text_size_for_width(ava_width)
                            }
                            (None, AvailableSpace::MinContent) => {
                                calculate_text_size_for_width(0.0)
                            }
                            (None, AvailableSpace::MaxContent) => {
                                calculate_text_size_for_width(f32::MAX)
                            }
                            (Some(act_width), _) => calculate_text_size_for_width(act_width),
                        },
                    )),
                )?
            }
            Node::Image(img) => {
                let img = store.get(img);
                let (width, height) = (img.width(), img.height());

                taffy.new_leaf_with_measure(
                    self.style.clone(),
                    MeasureFunc::Boxed(Box::new(
                        move |size: Size<Option<f32>>, available: Size<AvailableSpace>| match (
                            size.width,
                            available.width,
                        ) {
                            (None, AvailableSpace::Definite(ava_width)) => Size {
                                width: ava_width,
                                height: (ava_width / width as f32) * height as f32,
                            },
                            (None, AvailableSpace::MinContent) => Size::ZERO,
                            (None, AvailableSpace::MaxContent) => Size {
                                width: width as f32,
                                height: height as f32,
                            },
                            (Some(act_width), _) => Size {
                                width: act_width,
                                height: (act_width / width as f32) * height as f32,
                            },
                        },
                    )),
                )?
            }
            Node::Container(inner_children) => {
                children = Some(inner_children);

                taffy.new_leaf(self.style.clone())?
            }
        };

        taffy.add_child(parent, node)?;

        Ok((node, children))
    }

    fn into_draw_command(
        &self,
        layout: &Layout,
        store: &mut ImageStore,
    ) -> Option<(DrawCommand<'_>, u32)> {
        match &self.node {
            Node::TextCentered {
                text,
                font,
                scale,
                line_height,
                color,
            } => {
                let words = text.split(' ');
                let mut lines = Vec::default();

                let mut current_line = String::default();
                let mut current_width = 0.0;
                for word in words {
                    let mut temp_line = current_line.clone();

                    temp_line.push_str(word);

                    let width = font
                        .layout(&temp_line, *scale, rusttype::point(0.0, 0.0))
                        .last()
                        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
                        .unwrap_or(0.0);

                    if width > layout.size.width && !current_line.is_empty() {
                        let x_offset =
                            (layout.size.width - current_width) / 2.0 + layout.location.x;
                        lines.push((
                            UVec2 {
                                x: x_offset as u32,
                                y: ((*line_height * lines.len() as u32) as f32 + layout.location.y)
                                    as u32,
                            },
                            current_line,
                        ));
                        current_line = word.to_owned();
                        current_width = font
                            .layout(&current_line, *scale, rusttype::point(0.0, 0.0))
                            .last()
                            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
                            .unwrap_or(0.0);
                    } else {
                        current_line = temp_line;
                        current_width = width;
                    }

                    current_line.push(' ');
                }

                let x_offset = (layout.size.width - current_width) / 2.0 + layout.location.x;
                lines.push((
                    UVec2 {
                        x: x_offset as u32,
                        y: ((*line_height * lines.len() as u32) as f32 + layout.location.y) as u32,
                    },
                    current_line,
                ));

                Some((
                    DrawCommand::TextCentered {
                        lines,
                        font,
                        scale: *scale,
                        color: *color,
                    },
                    layout.order,
                ))
            }
            Node::Image(img) => {
                let img = store.get_resized(
                    img,
                    UVec2 {
                        x: layout.size.width as u32,
                        y: layout.size.height as u32,
                    },
                );

                Some((
                    DrawCommand::Image {
                        image: img.clone(),
                        position: UVec2 {
                            x: layout.location.x as u32,
                            y: layout.location.y as u32,
                        },
                    },
                    layout.order,
                ))
            }
            Node::Container(_) => None,
        }
    }
}

pub enum DrawCommand<'c> {
    FillBackground(Rgba<u8>),
    TextCentered {
        lines: Vec<(UVec2, String)>,
        font: &'c rusttype::Font<'c>,
        scale: rusttype::Scale,
        color: Rgba<u8>,
    },
    Image {
        image: RgbaImage,
        position: UVec2,
    },
}

impl<'c> DrawCommand<'c> {
    pub fn apply(&self, frame: &mut RgbaImage) -> anyhow::Result<()> {
        match self {
            DrawCommand::FillBackground(color) => {
                frame.pixels_mut().for_each(|pixel| *pixel = *color);
            }
            DrawCommand::TextCentered {
                lines,
                font,
                scale,
                color,
            } => {
                for (position, line) in lines {
                    imageproc::drawing::draw_text_mut(
                        frame,
                        *color,
                        position.x as i32,
                        position.y as i32,
                        *scale,
                        font,
                        line,
                    );
                }
            }
            DrawCommand::Image { image, position } => {
                image::imageops::overlay(frame, image, position.x as i64, position.y as i64);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VideoUI {
    pub children: Vec<StyledNode>,
    pub background_color: Rgba<u8>,
    image_store: ImageStore,
}

impl VideoUI {
    pub fn new(children: Vec<StyledNode>, background_color: Rgba<u8>) -> Self {
        VideoUI {
            children,
            background_color,
            image_store: ImageStore::default(),
        }
    }

    pub fn add(&mut self, img: RgbaImage) -> ImageHandle {
        self.image_store.add(img)
    }

    pub fn render(&mut self, frame: &mut RgbaImage) -> anyhow::Result<()> {
        let mut taffy = taffy::Taffy::new();

        let root = taffy.new_leaf(Style {
            flex_direction: taffy::style::FlexDirection::Column,
            size: Size {
                width: taffy::style::Dimension::Points(frame.width() as f32),
                height: taffy::style::Dimension::Points(frame.height() as f32),
            },
            position: Position::Absolute,
            // align_content: Some(AlignContent::Start),
            ..Default::default()
        })?;

        let mut elms = self
            .children
            .iter()
            .rev()
            .map(|child| (root, child))
            .collect::<Vec<_>>();
        let mut node_map: HashMap<_, _> = HashMap::default();

        while let Some((parent, styled_node)) = elms.pop() {
            let (node, children) = styled_node.process(&mut taffy, parent, &self.image_store)?;

            node_map.insert(node, styled_node);

            if let Some(children) = children {
                elms.extend(children.iter().map(|child| (node, child)).rev());
            }
        }

        taffy.compute_layout(root, Size::MAX_CONTENT)?;

        let mut commands = vec![(DrawCommand::FillBackground(self.background_color), 0)];
        let mut queued = vec![(Point { x: 0.0, y: 0.0 }, root)];

        while let Some((location, key)) = queued.pop() {
            let mut layout = taffy.layout(key)?.clone();
            layout.location.x += location.x;
            layout.location.y += location.y;

            if let Some(node) = node_map.get(&key) {
                if let Some(command) = node.into_draw_command(&layout, &mut self.image_store) {
                    commands.push(command);
                }
            }

            queued.extend(
                taffy
                    .children(key)?
                    .into_iter()
                    .map(|child| (layout.location, child)),
            );
        }

        commands.sort_by_key(|(_, key)| *key);

        for (command, _) in commands {
            command.apply(frame)?;
        }

        Ok(())
    }
}

impl Default for VideoUI {
    fn default() -> Self {
        Self {
            children: Default::default(),
            background_color: [0, 0, 0, 255].into(),
            image_store: Default::default(),
        }
    }
}

pub trait UiUpdater: Send + Sync + 'static {
    fn update(&mut self, frame_idx: u32, ui: &mut VideoUI);
}
