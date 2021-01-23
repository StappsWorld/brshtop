
#[derive(Clone)]
pub struct SubBox {
    box_x : u32,
    box_y : u32,
    box_width : u32,
    box_height : u32,
    box_columns : u32,
    column_size : u32,
} impl SubBox {

    pub fn new() -> Self {
        SubBox {
            box_x : 0,
            box_y : 0,
            box_width : 0,
            box_height : 0,
            box_columns : 0,
            column_size : 0,
        }
    }

    pub fn get_box_x(&self) -> u32 {
        self.box_x.clone()
    }

    pub fn set_box_x(&mut self, box_x : u32) {
        self.box_x = box_x.clone()
    }

    pub fn get_box_y(&self) -> u32 {
        self.box_y.clone()
    }

    pub fn set_box_y(&mut self, box_y : u32) {
        self.box_y = box_y.clone()
    }

    pub fn get_box_width(&self) -> u32 {
        self.box_width.clone()
    }

    pub fn set_box_width(&mut self, box_width : u32) {
        self.box_width = box_width.clone()
    }

    pub fn get_box_height(&self) -> u32 {
        self.box_height.clone()
    }

    pub fn set_box_height(&mut self, box_height : u32) {
        self.box_height = box_height.clone()
    }
    
    pub fn get_box_columns(&self) -> u32 {
        self.box_columns.clone()
    }

    pub fn set_box_columns(&mut self, box_columns : u32) {
        self.box_columns = box_columns.clone()
    }

    pub fn get_column_size(&self) -> u32 {
        self.column_size.clone()
    }

    pub fn set_column_size(&mut self, column_size : u32) {
        self.column_size = column_size.clone()
    }

}