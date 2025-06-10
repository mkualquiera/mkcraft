use tokio::task::JoinHandle;

pub const FRONT_X: f32 = 0.0;
pub const FRONT_Y: f32 = 0.0;
pub const FRONT_Z: f32 = 0.0;
pub const LEFT_X: f32 = 0.0;
pub const LEFT_Y: f32 = 0.0;
pub const LEFT_Z: f32 = 0.0;
pub const DOWN_X: f32 = 0.0;
pub const DOWN_Y: f32 = 0.0;
pub const DOWN_Z: f32 = 0.0;
pub const BACK_X: f32 = 0.0;
pub const BACK_Y: f32 = 0.0;
pub const BACK_Z: f32 = 1.0;
pub const RIGHT_X: f32 = 1.0;
pub const RIGHT_Y: f32 = 0.0;
pub const RIGHT_Z: f32 = 0.0;
pub const UP_X: f32 = 0.0;
pub const UP_Y: f32 = 1.0;
pub const UP_Z: f32 = 0.0;
pub const BACK_TOP_LEFT_X: f32 = BACK_X + UP_X + LEFT_X;
pub const BACK_TOP_LEFT_Y: f32 = BACK_Y + UP_Y + LEFT_Y;
pub const BACK_TOP_LEFT_Z: f32 = BACK_Z + UP_Z + LEFT_Z;
pub const BACK_TOP_RIGHT_X: f32 = BACK_X + UP_X + RIGHT_X;
pub const BACK_TOP_RIGHT_Y: f32 = BACK_Y + UP_Y + RIGHT_Y;
pub const BACK_TOP_RIGHT_Z: f32 = BACK_Z + UP_Z + RIGHT_Z;
pub const BACK_BOTTOM_LEFT_X: f32 = BACK_X + DOWN_X + LEFT_X;
pub const BACK_BOTTOM_LEFT_Y: f32 = BACK_Y + DOWN_Y + LEFT_Y;
pub const BACK_BOTTOM_LEFT_Z: f32 = BACK_Z + DOWN_Z + LEFT_Z;
pub const BACK_BOTTOM_RIGHT_X: f32 = BACK_X + DOWN_X + RIGHT_X;
pub const BACK_BOTTOM_RIGHT_Y: f32 = BACK_Y + DOWN_Y + RIGHT_Y;
pub const BACK_BOTTOM_RIGHT_Z: f32 = BACK_Z + DOWN_Z + RIGHT_Z;
pub const FRONT_TOP_LEFT_X: f32 = FRONT_X + UP_X + LEFT_X;
pub const FRONT_TOP_LEFT_Y: f32 = FRONT_Y + UP_Y + LEFT_Y;
pub const FRONT_TOP_LEFT_Z: f32 = FRONT_Z + UP_Z + LEFT_Z;
pub const FRONT_TOP_RIGHT_X: f32 = FRONT_X + UP_X + RIGHT_X;
pub const FRONT_TOP_RIGHT_Y: f32 = FRONT_Y + UP_Y + RIGHT_Y;
pub const FRONT_TOP_RIGHT_Z: f32 = FRONT_Z + UP_Z + RIGHT_Z;
pub const FRONT_BOTTOM_LEFT_X: f32 = FRONT_X + DOWN_X + LEFT_X;
pub const FRONT_BOTTOM_LEFT_Y: f32 = FRONT_Y + DOWN_Y + LEFT_Y;
pub const FRONT_BOTTOM_LEFT_Z: f32 = FRONT_Z + DOWN_Z + LEFT_Z;
pub const FRONT_BOTTOM_RIGHT_X: f32 = FRONT_X + DOWN_X + RIGHT_X;
pub const FRONT_BOTTOM_RIGHT_Y: f32 = FRONT_Y + DOWN_Y + RIGHT_Y;
pub const FRONT_BOTTOM_RIGHT_Z: f32 = FRONT_Z + DOWN_Z + RIGHT_Z;

pub enum QueuedItem<T> {
    Generating(JoinHandle<T>),
    Ready(T),
}

impl<T: Send + 'static> QueuedItem<T> {
    pub fn enqueue<F>(f: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        QueuedItem::Generating(tokio::spawn(f))
    }

    pub async fn get(&mut self) -> Option<&mut T> {
        match self {
            QueuedItem::Generating(handle) => {
                if handle.is_finished() {
                    let element = handle.await.expect("Failed to join handle");
                    *self = QueuedItem::Ready(element);
                    if let QueuedItem::Ready(item) = self {
                        return Some(item);
                    }
                    unreachable!();
                } else {
                    None
                }
            }
            QueuedItem::Ready(item) => return Some(item),
        }
    }
}
