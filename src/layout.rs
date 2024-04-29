pub use self::BoxType::{AnonymousBlock, BlockNode, InlineNode};
use crate::{css, style::StyledNode};
use css::Unit::Px;
use css::Value::{Keyword, Length};
use std::default::Default;

#[derive(Default, Debug)]
struct Dimensions {
    content: Rect,
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

#[derive(Default, Debug)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Default, Debug)]
struct EdgeSizes {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

struct LayoutBox<'a> {
    dimensions: Dimensions,
    box_type: BoxType<'a>,
    children: Vec<LayoutBox<'a>>,
}

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

    fn get_inline_container(&mut self) -> &mut LayoutBox {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            BlockNode(_) => match self.children.last() {
                Some(&LayoutBox {
                    box_type: AnonymousBlock,
                    ..
                }) => {}
                _ => self.children.push(LayoutBox::new(AnonymousBlock)),
            },
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
