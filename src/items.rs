use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ambient_api::prelude::*;

use crate::components::{crafting::*, player_hand_held_item_ref};

lazy_static::lazy_static! {
    pub static ref BLUE_ITEM: EntityId = Entity::new()
        .with(color(), vec4(0.0, 0.0, 1.0, 1.0))
        .spawn();

    pub static ref GREEN_ITEM: EntityId = Entity::new()
        .with(color(), vec4(0.0, 1.0, 0.0, 1.0))
        .spawn();

    pub static ref YELLOW_ITEM: EntityId = Entity::new()
        .with(color(), vec4(1.0, 1.0, 0.0, 1.0))
        .spawn();
}

/// Wasm-side crafting recipe data.
pub struct CraftingRecipe {
    pub recipe_entity: EntityId,
    pub primary_ingredient: EntityId,
    pub secondary_ingredient: Option<EntityId>,
    pub primary_yield: EntityId,
    pub secondary_yield: Option<EntityId>,
}

/// The set of all available crafting recipes.
pub struct RecipeStore {
    recipes: HashMap<(EntityId, Option<EntityId>), CraftingRecipe>,
}

impl RecipeStore {
    pub fn new() -> Self {
        Self {
            recipes: Default::default(),
        }
    }

    pub fn match_ingredients(
        &self,
        first_ingredient: EntityId,
        second_ingredient: EntityId,
    ) -> Option<(&CraftingRecipe, bool)> {
        if let Some(recipe) = self
            .recipes
            .get(&(first_ingredient, Some(second_ingredient)))
        {
            return Some((recipe, false));
        }

        if let Some(recipe) = self
            .recipes
            .get(&(second_ingredient, Some(first_ingredient)))
        {
            return Some((recipe, true));
        }

        if let Some(recipe) = self.recipes.get(&(first_ingredient, None)) {
            return Some((recipe, false));
        }

        if let Some(recipe) = self.recipes.get(&(second_ingredient, None)) {
            return Some((recipe, true));
        }

        None
    }

    pub fn apply_craft(
        &self,
        left_held: EntityId,
        right_held: EntityId,
    ) -> Option<(EntityId, EntityId)> {
        let (recipe, right_is_primary) = self.match_ingredients(left_held, right_held)?;

        if !right_is_primary {
            let new_second_held = if recipe.secondary_ingredient.is_none() {
                recipe.secondary_yield.unwrap_or(EntityId::null())
            } else {
                right_held
            };

            Some((recipe.primary_yield, new_second_held))
        } else {
            let new_first_held = if recipe.secondary_ingredient.is_none() {
                recipe.secondary_ingredient.unwrap_or(EntityId::null())
            } else {
                left_held
            };

            Some((new_first_held, recipe.primary_yield))
        }
    }
}

/// Sets up item- and crafting-related systems.
pub fn init_items() {
    let store = RecipeStore::new();
    let store = Arc::new(Mutex::new(store));

    spawn_query((recipe(), primary_ingredient(), primary_yield())).bind({
        let store = store.clone();
        move |recipes| {
            let mut store = store.lock().unwrap();
            for (e, (_recipe, mut primary_ingredient, primary_yield)) in recipes {
                let mut secondary_ingredient = entity::get_component(e, secondary_ingredient());
                let secondary_yield = entity::get_component(e, secondary_yield());

                // sort ingredient entity IDs to deduplicate swapped recipes
                if let Some(secondary_ingredient) = secondary_ingredient.as_mut() {
                    std::mem::swap(&mut primary_ingredient, secondary_ingredient);
                }

                let recipe = CraftingRecipe {
                    recipe_entity: e,
                    primary_ingredient,
                    secondary_ingredient,
                    primary_yield,
                    secondary_yield,
                };

                let recipe_key = (primary_ingredient, secondary_ingredient);

                if store.recipes.contains_key(&recipe_key) {
                    eprintln!("Duplicate crafting recipe");
                    continue;
                }

                store.recipes.insert(recipe_key, recipe);
            }
        }
    });

    // TODO these should run client-side.
    change_query(player_hand_held_item_ref())
        .track_change(player_hand_held_item_ref())
        .bind(move |changes| {
            for (e, item) in changes {
                let new_color =
                    entity::get_component(item, color()).unwrap_or(vec4(1.0, 0.0, 1.0, 1.0));

                entity::add_component(e, color(), new_color);
            }
        });

    // temp crafting recipe
    Entity::new()
        .with_default(recipe())
        .with(primary_ingredient(), *BLUE_ITEM)
        .with(secondary_ingredient(), *GREEN_ITEM)
        .with(primary_yield(), *YELLOW_ITEM)
        .spawn();
}
