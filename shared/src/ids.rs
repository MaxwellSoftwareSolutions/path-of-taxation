use serde::{Deserialize, Serialize};

/// Typed ID wrappers to prevent mixing different entity IDs.
macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub u64);

        impl $name {
            pub fn new(id: u64) -> Self {
                Self(id)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

typed_id!(CharacterId);
typed_id!(RunId);
typed_id!(RoomId);
typed_id!(EnemyInstanceId);
typed_id!(ItemId);
typed_id!(ProjectileId);
typed_id!(DebateSessionId);
