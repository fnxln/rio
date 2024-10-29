use crate::context::Context;
use crate::mouse::Mouse;
use rio_backend::crosswords::grid::Dimensions;
use rio_backend::event::EventListener;
use rio_backend::sugarloaf::{
    layout::SugarDimensions, Object, Rect, RichText, Sugarloaf,
};

const MIN_COLS: usize = 2;
const MIN_LINES: usize = 1;

const PADDING: f32 = 4.;

// $ tput columns
// $ tput lines
fn compute(
    width: f32,
    height: f32,
    dimensions: SugarDimensions,
    line_height: f32,
    margin: Delta<f32>,
) -> (usize, usize) {
    let margin_x = ((margin.x) * dimensions.scale).floor();
    let margin_spaces = margin.top_y + margin.bottom_y;

    let mut lines = (height / dimensions.scale) - margin_spaces;
    lines /= (dimensions.height / dimensions.scale) * line_height;
    let visible_lines = std::cmp::max(lines.floor() as usize, MIN_LINES);

    let mut visible_columns = (width / dimensions.scale) - margin_x;
    visible_columns /= dimensions.width / dimensions.scale;
    let visible_columns = std::cmp::max(visible_columns as usize, MIN_COLS);

    (visible_columns, visible_lines)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Delta<T: Default> {
    pub x: T,
    pub top_y: T,
    pub bottom_y: T,
}

pub struct ContextGrid<T: EventListener> {
    pub width: f32,
    pub height: f32,
    pub current: usize,
    pub margin: Delta<f32>,
    border_color: [f32; 4],
    inner: Vec<ContextGridItem<T>>,
}

pub struct ContextGridItem<T: EventListener> {
    val: Context<T>,
    right: Option<usize>,
    down: Option<usize>,
}

impl<T: rio_backend::event::EventListener> ContextGridItem<T> {
    pub fn new(context: Context<T>) -> Self {
        Self {
            val: context,
            right: None,
            down: None,
        }
    }
}

impl<T: rio_backend::event::EventListener> ContextGridItem<T> {
    #[inline]
    #[allow(unused)]
    pub fn context(&self) -> &Context<T> {
        &self.val
    }

    #[inline]
    pub fn context_mut(&mut self) -> &mut Context<T> {
        &mut self.val
    }
}

impl<T: rio_backend::event::EventListener> ContextGrid<T> {
    pub fn new(context: Context<T>, margin: Delta<f32>, border_color: [f32; 4]) -> Self {
        let width = context.dimension.width;
        let height = context.dimension.height;
        let inner = vec![ContextGridItem::new(context)];
        Self {
            inner,
            current: 0,
            margin,
            width,
            height,
            border_color,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn contexts_mut(&mut self) -> &mut Vec<ContextGridItem<T>> {
        &mut self.inner
    }

    #[inline]
    #[allow(unused)]
    pub fn contexts(&mut self) -> &Vec<ContextGridItem<T>> {
        &self.inner
    }

    #[inline]
    pub fn select_next_split(&mut self) {
        if self.inner.len() == 1 {
            return;
        }

        if self.current >= self.inner.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1;
        }
    }

    #[inline]
    pub fn select_prev_split(&mut self) {
        if self.inner.len() == 1 {
            return;
        }

        if self.current == 0 {
            self.current = self.inner.len() - 1;
        } else {
            self.current -= 1;
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn current_index(&self) -> usize {
        self.current
    }

    #[inline]
    pub fn current(&self) -> &Context<T> {
        &self.inner[self.current].val
    }

    #[inline]
    pub fn current_mut(&mut self) -> &mut Context<T> {
        &mut self.inner[self.current].val
    }

    #[inline]
    pub fn objects(&self) -> Vec<Object> {
        let len = self.inner.len();
        if len == 0 {
            return vec![];
        }

        let mut objects = Vec::with_capacity(len);

        // In case there's only 1 context then ignore quad
        if len == 1 {
            if let Some(item) = self.inner.first() {
                objects.push(Object::RichText(RichText {
                    id: item.val.rich_text_id,
                    position: [self.margin.x, self.margin.top_y],
                }));
            }
        } else {
            self.plot_objects(&mut objects, 0, self.margin);
        }
        objects
    }

    pub fn current_context_with_computed_dimension(&self) -> (&Context<T>, Delta<f32>) {
        let len = self.inner.len();
        if len <= 1 {
            return (&self.inner[self.current].val, self.margin);
        }

        let objects = self.objects();
        let rich_text_id = self.inner[self.current].val.rich_text_id;
        let mut margin = self.margin;
        for obj in objects {
            if let Object::RichText(rich_text_obj) = obj {
                if rich_text_obj.id == rich_text_id {
                    margin.x = rich_text_obj.position[0] + PADDING;
                    margin.top_y = rich_text_obj.position[1] + PADDING;
                    break;
                }
            }
        }

        (&self.inner[self.current].val, margin)
    }

    #[inline]
    pub fn select_current_based_on_mouse(&mut self, mouse: &Mouse) -> bool {
        let len = self.inner.len();
        if len <= 1 {
            return false;
        }

        let objects = self.objects();
        let mut select_new_current = None;
        for obj in objects {
            if let Object::RichText(rich_text_obj) = obj {
                if let Some(position) = self.find_by_rich_text_id(rich_text_obj.id) {
                    let scaled_position_x = rich_text_obj.position[0]
                        * self.inner[position].val.dimension.dimension.scale;
                    let scaled_position_y = rich_text_obj.position[1]
                        * self.inner[position].val.dimension.dimension.scale;
                    if mouse.x >= scaled_position_x as usize
                        && mouse.y >= scaled_position_y as usize
                    {
                        // println!("{:?} {:?} {:?}", mouse.x <= (scaled_position_x + self.inner[position].val.dimension.width) as usize, mouse.x, scaled_position_x + self.inner[position].val.dimension.width);
                        // println!("{:?} {:?} {:?}", mouse.y <= (scaled_position_y + self.inner[position].val.dimension.height) as usize, mouse.y, scaled_position_y + self.inner[position].val.dimension.height);
                        if mouse.x
                            <= (scaled_position_x
                                + self.inner[position].val.dimension.width)
                                as usize
                            && mouse.y
                                <= (scaled_position_y
                                    + self.inner[position].val.dimension.height)
                                    as usize
                        {
                            select_new_current = Some(position);
                            break;
                        }
                    }
                }
            }
        }

        if let Some(new_current) = select_new_current {
            self.current = new_current;
            return true;
        }

        false
    }

    pub fn find_by_rich_text_id(&self, searched_rich_text_id: usize) -> Option<usize> {
        self.inner
            .iter()
            .position(|context| context.val.rich_text_id == searched_rich_text_id)
    }

    #[inline]
    pub fn grid_dimension(&self) -> ContextDimension {
        let current_context_dimension = self.inner[self.current].val.dimension;
        ContextDimension::build(
            self.width,
            self.height,
            current_context_dimension.dimension,
            1.0,
            self.margin,
        )
    }

    pub fn plot_objects(
        &self,
        objects: &mut Vec<Object>,
        index: usize,
        margin: Delta<f32>,
    ) {
        if let Some(item) = self.inner.get(index) {
            objects.push(Object::RichText(RichText {
                id: item.val.rich_text_id,
                position: [margin.x, margin.top_y],
            }));

            if let Some(right_item) = item.right {
                let new_margin = Delta {
                    x: margin.x
                        + PADDING
                        + (item.val.dimension.width / item.val.dimension.dimension.scale),
                    top_y: margin.top_y,
                    bottom_y: margin.bottom_y,
                };

                objects.push(Object::Rect(Rect {
                    position: [new_margin.x - PADDING, new_margin.top_y],
                    color: self.border_color,
                    size: [
                        2. / item.val.dimension.dimension.scale,
                        item.val.dimension.height,
                    ],
                }));

                self.plot_objects(objects, right_item, new_margin);
            }

            if let Some(down_item) = item.down {
                let new_margin = Delta {
                    x: margin.x,
                    top_y: margin.top_y
                        + PADDING
                        + (item.val.dimension.height
                            / item.val.dimension.dimension.scale),
                    bottom_y: margin.bottom_y,
                };

                objects.push(Object::Rect(Rect {
                    position: [new_margin.x, new_margin.top_y - PADDING],
                    color: self.border_color,
                    size: [
                        item.val.dimension.width,
                        2. / item.val.dimension.dimension.scale,
                    ],
                }));

                self.plot_objects(objects, down_item, new_margin);
            }
        }
    }

    pub fn update_margin(&mut self, padding: (f32, f32, f32)) {
        self.margin = Delta {
            x: padding.0,
            top_y: padding.1,
            bottom_y: padding.2,
        };
        for context in &mut self.inner {
            context.val.dimension.update_margin(self.margin);
        }
    }

    pub fn update_dimensions(&mut self, sugarloaf: &Sugarloaf) {
        for context in &mut self.inner {
            let layout = sugarloaf.rich_text_layout(&context.val.rich_text_id);
            context.val.dimension.update_dimensions(layout.dimensions);
        }
    }

    pub fn resize(&mut self, new_width: f32, new_height: f32) {
        let width_difference = new_width - self.width;
        let height_difference = new_height - self.height;
        self.width = new_width;
        self.height = new_height;

        let mut vector = vec![(0., 0.); self.inner.len()];
        self.resize_context(&mut vector, 0, width_difference, height_difference);

        for (index, val) in vector.into_iter().enumerate() {
            let context = &mut self.inner[index];
            let current_width = context.val.dimension.width;
            context.val.dimension.update_width(current_width + val.0);

            let current_height = context.val.dimension.height;
            context.val.dimension.update_height(current_height + val.1);

            let mut terminal = context.val.terminal.lock();
            terminal.resize::<ContextDimension>(context.val.dimension);
            drop(terminal);
            let winsize =
                crate::renderer::utils::terminal_dimensions(&context.val.dimension);
            let _ = context.val.messenger.send_resize(winsize);
        }
    }

    // TODO: It works partially, if the panels have different dimensions it gets a bit funky
    fn resize_context(
        &self,
        vector: &mut Vec<(f32, f32)>,
        index: usize,
        available_width: f32,
        available_height: f32,
    ) -> (f32, f32) {
        if let Some(item) = self.inner.get(index) {
            let mut current_available_width = available_width;
            let mut current_available_heigth = available_height;
            if let Some(right_item) = item.right {
                let (new_available_width, _) = self.resize_context(
                    vector,
                    right_item,
                    available_width / 2.,
                    available_height,
                );
                current_available_width = new_available_width;
            }

            if let Some(down_item) = item.down {
                let (_, new_available_heigth) = self.resize_context(
                    vector,
                    down_item,
                    available_width,
                    available_height / 2.,
                );
                current_available_heigth = new_available_heigth;
            }

            vector[index] = (current_available_width, current_available_heigth);

            return (current_available_width, current_available_heigth);
        }

        (available_width, available_height)
    }

    fn request_resize(&mut self, index: usize) {
        let mut terminal = self.inner[index].val.terminal.lock();
        terminal.resize::<ContextDimension>(self.inner[index].val.dimension);
        drop(terminal);
        let winsize =
            crate::renderer::utils::terminal_dimensions(&self.inner[index].val.dimension);
        let _ = self.inner[index].val.messenger.send_resize(winsize);
    }

    pub fn remove_current(&mut self) {
        let mut parent_context = None;
        for (index, context) in self.inner.iter().enumerate() {
            if let Some(right_val) = context.right {
                if right_val == self.current {
                    parent_context = Some((true, index));
                    break;
                }
            }

            if let Some(down_val) = context.down {
                if down_val == self.current {
                    parent_context = Some((false, index));
                    break;
                }
            }
        }

        let to_be_removed = self.current;
        let to_be_removed_width = self.inner[to_be_removed].val.dimension.width;
        let to_be_removed_height = self.inner[to_be_removed].val.dimension.height;

        // If index to be removed is owned by a parent context
        if let Some((is_right, parent_index)) = parent_context {
            let mut next_current = parent_index;
            if is_right {
                // If current has down items then need to inherit
                if let Some(current_down) = self.inner[self.current].down {
                    self.inner[current_down]
                        .val
                        .dimension
                        .increase_height(to_be_removed_height + PADDING);

                    self.request_resize(current_down);

                    next_current = current_down.wrapping_sub(1);
                    self.inner[parent_index].right = Some(next_current);

                // If current has no down items then check right items to inherit
                } else {
                    let parent_width = self.inner[parent_index].val.dimension.width;
                    self.inner[parent_index]
                        .val
                        .dimension
                        .update_width(parent_width + to_be_removed_width + PADDING);
                    self.inner[parent_index].right = None;

                    if let Some(current_right) = self.inner[self.current].right {
                        self.inner[parent_index].right = Some(current_right);
                    }

                    self.request_resize(parent_index);
                }
            } else {
                // If current has right items then need to inherit
                if let Some(current_right) = self.inner[self.current].right {
                    self.inner[current_right]
                        .val
                        .dimension
                        .increase_width(to_be_removed_width + PADDING);

                    self.request_resize(current_right);

                    next_current = current_right.wrapping_sub(1);
                    self.inner[parent_index].down = Some(next_current);

                // If current has no right items then check right items to inherit
                } else {
                    let parent_height = self.inner[parent_index].val.dimension.height;
                    self.inner[parent_index]
                        .val
                        .dimension
                        .update_height(parent_height + to_be_removed_height + PADDING);
                    self.inner[parent_index].down = None;

                    // If current has down items then need to inherit
                    if let Some(current_down) = self.inner[self.current].down {
                        self.inner[parent_index].down = Some(current_down);
                    }

                    self.request_resize(parent_index);
                }
            }

            self.remove_index(to_be_removed);
            self.current = next_current;
            return;
        }

        // In case there is no parenting, needs to validate if it has children
        if let Some(right_val) = self.inner[to_be_removed].right {
            let right_width = self.inner[right_val].val.dimension.width;
            self.inner[right_val]
                .val
                .dimension
                .update_width(right_width + to_be_removed_width + PADDING);

            self.request_resize(right_val);
            self.remove_index(to_be_removed);
            return;
        }

        if let Some(down_val) = self.inner[to_be_removed].down {
            let down_height = self.inner[down_val].val.dimension.height;
            self.inner[down_val]
                .val
                .dimension
                .update_height(down_height + to_be_removed_height + PADDING);

            self.request_resize(down_val);
            self.remove_index(to_be_removed);
            return;
        }

        self.select_prev_split();
        self.remove_index(to_be_removed);
    }

    fn remove_index(&mut self, index: usize) {
        for context in &mut self.inner {
            if let Some(right_val) = context.right {
                if right_val > index {
                    context.right = Some(right_val.wrapping_sub(1));
                }
            }

            if let Some(down_val) = context.down {
                if down_val > index {
                    context.down = Some(down_val.wrapping_sub(1));
                }
            }
        }
        self.inner.remove(index);
    }

    pub fn split_right(&mut self, context: Context<T>) {
        let old_right = self.inner[self.current].right;

        let old_grid_item_height = self.inner[self.current].val.dimension.height;
        let old_grid_item_width = self.inner[self.current].val.dimension.width;
        let new_grid_item_width = old_grid_item_width / 2.0;
        self.inner[self.current]
            .val
            .dimension
            .update_width(new_grid_item_width - PADDING);
        self.request_resize(self.current);

        let mut new_context = ContextGridItem::new(context);

        new_context.val.dimension.update_width(new_grid_item_width);
        new_context
            .val
            .dimension
            .update_height(old_grid_item_height);

        self.inner.push(new_context);
        let new_current = self.inner.len() - 1;

        self.request_resize(new_current);
        self.inner[new_current].right = old_right;
        self.inner[self.current].right = Some(new_current);
        self.current = new_current;
    }

    pub fn split_down(&mut self, context: Context<T>) {
        let old_down = self.inner[self.current].down;

        let old_grid_item_height = self.inner[self.current].val.dimension.height;
        let old_grid_item_width = self.inner[self.current].val.dimension.width;
        let new_grid_item_height = old_grid_item_height / 2.0;
        self.inner[self.current]
            .val
            .dimension
            .update_height(new_grid_item_height - PADDING);
        self.request_resize(self.current);

        let mut new_context = ContextGridItem::new(context);
        new_context
            .val
            .dimension
            .update_height(new_grid_item_height);
        new_context.val.dimension.update_width(old_grid_item_width);

        self.inner.push(new_context);
        let new_current = self.inner.len() - 1;
        self.request_resize(new_current);

        self.inner[new_current].down = old_down;
        self.inner[self.current].down = Some(new_current);
        self.current = new_current;
    }
}

#[derive(Copy, Clone)]
pub struct ContextDimension {
    pub width: f32,
    pub height: f32,
    pub columns: usize,
    pub lines: usize,
    pub dimension: SugarDimensions,
    pub margin: Delta<f32>,
}

impl Default for ContextDimension {
    fn default() -> ContextDimension {
        ContextDimension {
            width: 0.,
            height: 0.,
            columns: MIN_COLS,
            lines: MIN_LINES,
            dimension: SugarDimensions::default(),
            margin: Delta::<f32>::default(),
        }
    }
}

impl ContextDimension {
    pub fn build(
        width: f32,
        height: f32,
        dimension: SugarDimensions,
        line_height: f32,
        margin: Delta<f32>,
    ) -> Self {
        let (columns, lines) = compute(width, height, dimension, line_height, margin);
        Self {
            width,
            height,
            columns,
            lines,
            dimension,
            margin,
        }
    }

    #[inline]
    pub fn update_width(&mut self, width: f32) {
        self.width = width;
        self.update();
    }

    #[inline]
    pub fn increase_width(&mut self, acc_width: f32) {
        self.width += acc_width;
        self.update();
    }

    #[inline]
    pub fn update_height(&mut self, height: f32) {
        self.height = height;
        self.update();
    }

    #[inline]
    pub fn increase_height(&mut self, acc_height: f32) {
        self.height += acc_height;
        self.update();
    }

    #[inline]
    pub fn update_margin(&mut self, margin: Delta<f32>) {
        self.margin = margin;
        self.update();
    }

    #[inline]
    pub fn update_dimensions(&mut self, dimensions: SugarDimensions) {
        self.dimension = dimensions;
        self.update();
    }

    #[inline]
    fn update(&mut self) {
        let (columns, lines) = compute(
            self.width,
            self.height,
            self.dimension,
            // self.line_height,
            1.0,
            self.margin,
        );

        self.columns = columns;
        self.lines = lines;
    }
}

impl Dimensions for ContextDimension {
    #[inline]
    fn columns(&self) -> usize {
        self.columns
    }

    #[inline]
    fn screen_lines(&self) -> usize {
        self.lines
    }

    #[inline]
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn square_width(&self) -> f32 {
        self.dimension.width
    }

    fn square_height(&self) -> f32 {
        self.dimension.height
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::context::create_mock_context;
    use crate::event::VoidListener;
    use rio_window::window::WindowId;

    #[test]
    fn test_single_context_respecting_margin_and_no_quad_creation() {
        let margin = Delta {
            x: 10.,
            top_y: 20.,
            bottom_y: 20.,
        };

        let context_dimension = ContextDimension::build(
            1200.0,
            800.0,
            SugarDimensions {
                scale: 2.,
                width: 18.,
                height: 9.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 66);
        assert_eq!(context_dimension.lines, 88);
        let rich_text_id = 1;
        let route_id = 0;
        let context = create_mock_context(
            VoidListener {},
            WindowId::from(0),
            route_id,
            rich_text_id,
            context_dimension,
        );
        let context_width = context.dimension.width;
        let context_height = context.dimension.height;
        let context_margin = context.dimension.margin;
        let grid = ContextGrid::<VoidListener>::new(context, margin, [0., 0., 0., 0.]);
        // The first context should fill completely w/h grid
        assert_eq!(grid.width, context_width);
        assert_eq!(grid.height, context_height);

        // Context margin should empty
        assert_eq!(Delta::<f32>::default(), context_margin);
        assert_eq!(grid.margin, margin);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: rich_text_id,
                position: [10., 20.],
            })]
        );
    }

    #[test]
    fn test_split_right() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            1200.0,
            800.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 85);
        assert_eq!(context_dimension.lines, 100);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [1., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );
        grid.split_right(second_context);

        assert_eq!(
            grid.objects(),
            vec![
                Object::RichText(RichText {
                    id: first_context_id,
                    position: [0.0, 0.0],
                }),
                Object::Rect(Rect {
                    position: [298.0, 0.0],
                    color: [1.0, 0.0, 0.0, 0.0],
                    size: [1.0, 800.0]
                }),
                Object::RichText(RichText {
                    id: second_context_id,
                    position: [302.0, 0.0]
                }),
            ]
        );

        let (third_context, third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        grid.split_right(third_context);

        assert_eq!(
            grid.objects(),
            vec![
                Object::RichText(RichText {
                    id: first_context_id,
                    position: [0.0, 0.0],
                }),
                Object::Rect(Rect {
                    position: [298.0, 0.0],
                    color: [1.0, 0.0, 0.0, 0.0],
                    size: [1.0, 800.0]
                }),
                Object::RichText(RichText {
                    id: second_context_id,
                    position: [302.0, 0.0]
                }),
                Object::Rect(Rect {
                    position: [450.0, 0.0],
                    color: [1.0, 0.0, 0.0, 0.0],
                    size: [1.0, 800.0]
                }),
                Object::RichText(RichText {
                    id: third_context_id,
                    position: [454.0, 0.0]
                }),
            ]
        );
    }

    #[test]
    fn test_split_down() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            1200.0,
            800.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 85);
        assert_eq!(context_dimension.lines, 100);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 1., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );
        grid.split_down(second_context);

        assert_eq!(
            grid.objects(),
            vec![
                Object::RichText(RichText {
                    id: first_context_id,
                    position: [0.0, 0.0],
                }),
                Object::Rect(Rect {
                    position: [0.0, 198.0],
                    color: [0.0, 0.0, 1.0, 0.0],
                    size: [1200.0, 1.0]
                }),
                Object::RichText(RichText {
                    id: second_context_id,
                    position: [0.0, 202.0]
                }),
            ]
        );

        let (third_context, third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        grid.split_down(third_context);

        assert_eq!(
            grid.objects(),
            vec![
                Object::RichText(RichText {
                    id: first_context_id,
                    position: [0.0, 0.0],
                }),
                Object::Rect(Rect {
                    position: [0.0, 198.0],
                    color: [0.0, 0.0, 1.0, 0.0],
                    size: [1200.0, 1.0]
                }),
                Object::RichText(RichText {
                    id: second_context_id,
                    position: [0.0, 202.0]
                }),
                Object::Rect(Rect {
                    position: [0.0, 300.0],
                    color: [0.0, 0.0, 1.0, 0.0],
                    size: [1200.0, 1.0]
                }),
                Object::RichText(RichText {
                    id: third_context_id,
                    position: [0.0, 304.0]
                }),
            ]
        );
    }

    #[test]
    fn test_resize() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, _second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (third_context, _third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.split_right(second_context);
        grid.split_down(third_context);

        // assert_eq!(
        //     grid.objects(),
        //     vec![
        //         Object::RichText(RichText {
        //             id: first_context_id,
        //             position: [0.0, 0.0],
        //         }),
        //         Object::Rect(Rect {
        //             position: [147.0, 0.0],
        //             color: [0.0, 0.0, 0.0, 0.0],
        //             size: [1.0, 300.0]
        //         }),
        //         Object::RichText(RichText {
        //             id: second_context_id,
        //             position: [149.0, 0.0]
        //         }),
        //         Object::Rect(Rect {
        //             position: [149.0, 147.0],
        //             color: [0.0, 0.0, 0.0, 0.0],
        //             size: [294.0, 1.0]
        //         }),
        //         Object::RichText(RichText {
        //             id: third_context_id,
        //             position: [149.0, 149.0]
        //         }),
        //     ]
        // );

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        grid.resize(1200.0, 600.0);

        // TODO: Finish test
    }

    #[test]
    fn test_remove_right_without_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, _second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);
        assert_eq!(grid.current().dimension.width, 600.);

        grid.split_right(second_context);

        let new_expected_width = 600. / 2.;

        assert_eq!(grid.current().dimension.width, new_expected_width);
        assert_eq!(grid.current_index(), 1);

        grid.select_prev_split();
        let old_expected_width = (600. / 2.) - PADDING;
        assert_eq!(grid.current().dimension.width, old_expected_width);
        assert_eq!(grid.current_index(), 0);

        grid.select_next_split();
        assert_eq!(grid.current_index(), 1);

        grid.remove_current();

        assert_eq!(grid.current_index(), 0);
        // Whenever return to one should drop padding
        assert_eq!(grid.current().dimension.width, 600.);
    }

    #[test]
    fn test_remove_right_with_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, _second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.split_right(second_context);

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        let new_context_expected_width = 600. / 2.;

        assert_eq!(grid.current().dimension.width, new_context_expected_width);
        assert_eq!(grid.current_index(), 1);

        grid.select_prev_split();

        let old_context_expected_width = (600. / 2.) - PADDING;
        assert_eq!(grid.current().dimension.width, old_context_expected_width);
        assert_eq!(grid.current_index(), 0);

        let current_index = grid.current_index();
        assert_eq!(grid.contexts()[current_index].right, Some(1));
        assert_eq!(grid.contexts()[current_index].down, None);

        grid.remove_current();

        assert_eq!(grid.current_index(), 0);
        // Whenever return to one should drop padding
        let expected_width = 600.;
        assert_eq!(grid.current().dimension.width, expected_width);

        let current_index = grid.current_index();
        assert_eq!(grid.contexts()[current_index].right, None);
        assert_eq!(grid.contexts()[current_index].down, None);
    }

    #[test]
    fn test_remove_right_with_down_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.split_right(second_context);

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        let new_context_expected_width = 600. / 2.;

        assert_eq!(grid.current().dimension.width, new_context_expected_width);
        assert_eq!(grid.current_index(), 1);

        let (third_context, third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        grid.split_down(third_context);
        assert_eq!(grid.current_index(), 2);
        assert_eq!(grid.current().dimension.width, new_context_expected_width);
        assert_eq!(grid.current().dimension.height, 300.);

        // Move back
        grid.select_prev_split();

        assert_eq!(grid.current_index(), 1);
        assert_eq!(grid.current().rich_text_id, second_context_id);
        assert_eq!(grid.current().dimension.width, new_context_expected_width);
        assert_eq!(grid.current().dimension.height, 296.);

        // Remove the current should actually make right being down
        grid.remove_current();

        assert_eq!(grid.current_index(), 1);
        assert_eq!(grid.current().rich_text_id, third_context_id);
        assert_eq!(grid.current().dimension.width, new_context_expected_width);
        assert_eq!(grid.current().dimension.height, 600.);
    }

    #[test]
    fn test_remove_down_without_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, _second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);
        assert_eq!(grid.current().dimension.width, 600.);

        grid.split_down(second_context);

        let new_expected_width = 600. / 2.;

        assert_eq!(grid.current().dimension.height, new_expected_width);
        assert_eq!(grid.current_index(), 1);

        grid.select_prev_split();
        let old_expected_width = (600. / 2.) - PADDING;
        assert_eq!(grid.current().dimension.height, old_expected_width);
        assert_eq!(grid.current_index(), 0);

        grid.select_next_split();
        assert_eq!(grid.current_index(), 1);

        grid.remove_current();

        assert_eq!(grid.current_index(), 0);
        // Whenever return to one should drop padding
        assert_eq!(grid.current().dimension.height, 600.);
    }

    #[test]
    fn test_remove_down_with_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, _second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.split_down(second_context);

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        let new_context_expected_height = 600. / 2.;

        assert_eq!(grid.current().dimension.height, new_context_expected_height);
        assert_eq!(grid.current_index(), 1);

        grid.select_prev_split();

        let old_context_expected_height = (600. / 2.) - PADDING;
        assert_eq!(grid.current().dimension.height, old_context_expected_height);
        assert_eq!(grid.current_index(), 0);

        let current_index = grid.current_index();
        assert_eq!(grid.contexts()[current_index].down, Some(1));
        assert_eq!(grid.contexts()[current_index].right, None);

        grid.remove_current();

        assert_eq!(grid.current_index(), 0);
        // Whenever return to one should drop padding
        let expected_height = 600.;
        assert_eq!(grid.current().dimension.height, expected_height);

        let current_index = grid.current_index();
        assert_eq!(grid.contexts()[current_index].down, None);
        assert_eq!(grid.contexts()[current_index].right, None);
    }

    #[test]
    fn test_remove_down_with_right_children() {
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.split_down(second_context);

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        let new_context_expected_height = 600. / 2.;

        assert_eq!(grid.current().dimension.height, new_context_expected_height);
        assert_eq!(grid.current_index(), 1);

        let (third_context, third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        grid.split_right(third_context);
        assert_eq!(grid.current_index(), 2);
        assert_eq!(grid.current().dimension.width, new_context_expected_height);
        assert_eq!(grid.current().dimension.height, 300.);

        // Move back
        grid.select_prev_split();

        assert_eq!(grid.current_index(), 1);
        assert_eq!(grid.current().rich_text_id, second_context_id);
        assert_eq!(grid.current().dimension.height, new_context_expected_height);
        assert_eq!(grid.current().dimension.width, 296.);

        // Remove the current should actually make down being down
        grid.remove_current();

        assert_eq!(grid.current_index(), 1);
        assert_eq!(grid.current().rich_text_id, third_context_id);
        assert_eq!(grid.current().dimension.height, new_context_expected_height);
        assert_eq!(grid.current().dimension.width, 600.);
    }

    #[test]
    fn test_select_current_based_on_mouse() {
        let mut mouse = Mouse::default();
        let margin = Delta {
            x: 0.,
            top_y: 0.,
            bottom_y: 0.,
        };

        let context_dimension = ContextDimension::build(
            600.0,
            600.0,
            SugarDimensions {
                scale: 2.,
                width: 14.,
                height: 8.,
            },
            1.0,
            Delta::<f32>::default(),
        );

        assert_eq!(context_dimension.columns, 42);
        assert_eq!(context_dimension.lines, 75);

        let (first_context, first_context_id) = {
            let rich_text_id = 0;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let (second_context, second_context_id) = {
            let rich_text_id = 1;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        let mut grid =
            ContextGrid::<VoidListener>::new(first_context, margin, [0., 0., 0., 0.]);

        assert_eq!(
            grid.objects(),
            vec![Object::RichText(RichText {
                id: first_context_id,
                position: [0., 0.],
            })]
        );

        grid.select_current_based_on_mouse(&mouse);
        // On first should always return first item
        assert_eq!(grid.current_index(), 0);

        grid.split_down(second_context);

        assert_eq!(grid.width, 600.0);
        assert_eq!(grid.height, 600.0);

        let new_context_expected_height = 600. / 2.;

        assert_eq!(grid.current().dimension.height, new_context_expected_height);
        assert_eq!(grid.current_index(), 1);

        let (third_context, third_context_id) = {
            let rich_text_id = 2;
            let route_id = 0;
            (
                create_mock_context(
                    VoidListener {},
                    WindowId::from(0),
                    route_id,
                    rich_text_id,
                    context_dimension,
                ),
                rich_text_id,
            )
        };

        grid.split_right(third_context);
        assert_eq!(grid.current_index(), 2);
        assert_eq!(grid.current().dimension.width, new_context_expected_height);
        assert_eq!(grid.current().dimension.height, 300.);

        grid.select_current_based_on_mouse(&mouse);
        assert_eq!(grid.current_index(), 0);
        assert_eq!(grid.current().rich_text_id, 0);

        mouse.y = (new_context_expected_height + PADDING) as usize;
        grid.select_current_based_on_mouse(&mouse);

        assert_eq!(grid.current_index(), 1);
        assert_eq!(grid.current().rich_text_id, second_context_id);

        mouse.x = 304;
        grid.select_current_based_on_mouse(&mouse);

        assert_eq!(grid.current_index(), 2);
        assert_eq!(grid.current().rich_text_id, third_context_id);
    }
}
