

pub struct SubBox {
    pub box_x : u32,
    pub box_y : u32,
    pub box_width : u32,
    pub box_height : u32,
    pub box_columns : u32,
    pub column_size : u32,
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
}