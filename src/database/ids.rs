macro_rules! database_id {
    ($i:ident) => {
        #[derive(sqlx::Type, Copy, Clone, Debug, Eq, Hash, PartialEq)]
        #[sqlx(transparent)]
        pub struct $i(pub i64);

        impl $i {
            pub fn into_serenity(self) -> poise::serenity_prelude::model::id::$i {
                (unsafe {::core::num::NonZero::new_unchecked(self.0 as u64) }).into()
            }
        }

        impl From<$i> for poise::serenity_prelude::model::id::$i {
            fn from(id: $i) -> Self {
                id.into_serenity()
            }
        }

        impl From<poise::serenity_prelude::model::id::$i> for $i {
            fn from(id: poise::serenity_prelude::model::id::$i) -> Self {
                Self(id.get() as i64)
            }
        }
    };
}

database_id!(ChannelId);
database_id!(MessageId);
database_id!(GuildId);
database_id!(UserId);
