pub use self::BoxType::{AnonymousBlock, BlockNode, InlineNode};
use crate::{
    css,
    style::{
        Display::{Block, Inline, None},
        StyledNode,
    },
};
use css::Unit::Px;
use css::Value::{Keyword, Length};
use std::default::Default;

#[derive(Default, Debug, Clone)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    // paddingの大きさ分足す
    fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    // paddingの大きさ + borderの太さ分足す
    fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }

    // paddingの大きさ + borderの太さ + marginの大きさ分足す
    fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[derive(Default, Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Copy for Rect {}
impl Copy for Dimensions {}
impl Copy for EdgeSizes {}

#[derive(Debug)]
pub struct LayoutBox<'a> {
    dimensions: Dimensions,
    box_type: BoxType<'a>,
    children: Vec<LayoutBox<'a>>,
}

#[derive(Debug)]
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) | InlineNode(node) => node,
            AnonymousBlock => panic!("Anonymous block box has no style node"),
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            // 匿名ブロックボックスがあればそれを使い、なければ新しく作成する。
            BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) => {}
            AnonymousBlock => {}
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // 子要素の幅は親要素によって決まるので、先に親要素の幅を計算する
        self.calculate_block_width(containing_block);

        // コンテナー内のどこに設置するか計算する
        self.calculate_block_position(containing_block);

        // 再帰的に子要素もレイアウトする
        self.layout_block_children();

        // 親要素の高さは子要素の高さによって決まるので子要素が設置された後に高さを計算する
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin,border, paddginの初期値
        let zero = Length(0.0, Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let mut padding_left = style.lookup("padding-left", "margin", &zero);
        let mut padding_right = style.lookup("padding-right", "margin", &zero);

        let total = sum([
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ]
        .iter()
        .map(|v| v.to_px()));

        // 子要素の幅が親要素より大きければmarginを0に調整する
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }

            if margin_right == auto {
                margin_right = Length(0.0, Px);
            }
        }

        // 空いてるスペース
        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // どれもautoではない場合、margin_rightで調整する
            (false, false, false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }

            // 左右のmarginのどちらかがautoだった場合、autoになっている箇所で調整する
            (false, false, true) => {
                margin_right = Length(underflow, Px);
            }
            (false, true, false) => {
                margin_left = Length(underflow, Px);
            }

            // widthがautoだったら他の値を0にする
            (true, _, _) => {
                if margin_left == auto {
                    margin_left == Length(0.0, Px);
                }
                if margin_right == auto {
                    margin_right == Length(0.0, Px);
                }

                if underflow >= 0.0 {
                    // underflowが正の時はその値をwidthに設定する
                    width = Length(underflow, Px);
                } else {
                    // 負だった場合はmargin-rightから引いて調整する
                    width = Length(0.0, Px);
                    margin_right = Length(margin_right.to_px() + underflow, Px)
                }
            }

            // margin-leftとmargin-rightの両方ともautoだったらそれぞれにunderflowの半分を設定する
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        let zero = Length(0.0, Px);

        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing_block.content.height
            + containing_block.content.y
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(*d);
            d.content.height = d.content.height + child.dimensions.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        // heightプロパティが設定されていればそれを使う
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h
        }
    }
}

/// Transform a style tree into a layout tree.
pub fn layout_tree<'a>(
    node: &'a StyledNode<'a>,
    mut containing_block: Dimensions,
) -> LayoutBox<'a> {
    // The layout algorithm expects the container height to start at 0.
    // TODO: Save the initial containing block height, for calculating percent heights.
    containing_block.content.height = 0.0;

    let mut root_box = build_layout_tree(node);
    root_box.layout(containing_block);
    root_box
}

fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Block => BlockNode(style_node),
        Inline => InlineNode(style_node),
        DisplayNone => panic!("Root node has display: none"),
    });

    for child in &style_node.children {
        match child.display() {
            Block => root.children.push(build_layout_tree(child)),
            Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            DisplayNone => {}
        }
    }

    root
}

fn sum<I>(iter: I) -> f32
where
    I: Iterator<Item = f32>,
{
    iter.fold(0., |a, b| a + b)
}
