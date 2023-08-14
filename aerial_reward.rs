/// use this to wrap a regular reward like vel ball to goal in a height modifier instead of building a custom version
pub struct AerialWeightedWrapper {
    // reward fn that is called and applied the ratio to
    reward_fn: Box<dyn RewardFn + Send>,
    // maximum that the reward fn can be multiplied by
    max_height_ratio: f32,
    // minimum that the reward fn can be multiplied by
    min_height_ratio: f32,
    // the calculated target height after every match
    target_height: f32,
    // the maximum height that target_height can get to
    max_height: f32,
    // minimum that target_height can get to
    min_height: f32,
    // used to calculate average height from previous match
    total_height: f32,
    // divisor when calculating average height
    num_ticks_touched: u64,
    // used to check so that we only add each tick once to the total_height
    curr_tick: u64,
}

impl AerialWeightedWrapper {
    pub fn new(
        reward_fn: Box<dyn RewardFn + Send>,
        min_height_val: Option<f32>,
        max_height_val: Option<f32>,
        min_val_ratio: Option<f32>,
        max_val_ratio: Option<f32>,
    ) -> Self {
        let min_height_ratio = min_val_ratio.unwrap_or(0.1);
        let max_height_ratio = max_val_ratio.unwrap_or(4.0);
        let min_height = min_height_val.unwrap_or(150.);
        let max_height = max_height_val.unwrap_or(800.);
        AerialWeightedWrapper {
            reward_fn,
            max_height_ratio,
            min_height_ratio,
            target_height: min_height,
            max_height,
            min_height,
            total_height: 0.,
            num_ticks_touched: 0,
            curr_tick: 0,
        }
    }
}

impl RewardFn for AerialWeightedWrapper {
    fn reset(&mut self, initial_state: &GameState) {
        let mut avg_height = self.total_height / self.num_ticks_touched as f32;
        // this should be only in the case of a restart where it would be 0/0
        if avg_height.is_nan() {
            avg_height = self.min_height;
        }
        self.target_height = if avg_height > self.max_height {
            self.max_height
        } else if avg_height < self.min_height {
            self.min_height
        } else if avg_height < self.target_height {
            let delta = self.target_height - avg_height;
            // If we allow target_height to drop immediately with the delta then there's a chance the algorithm may try to keep the target height low to get better rewards (not tested though)
            self.target_height - delta * 0.05
        } else {
            let delta = avg_height - self.target_height;
            // We also don't want the average to jump too fast in case of a short match like a replay setter state
            self.target_height + delta * 0.25
        };

        self.num_ticks_touched = 0;
        self.curr_tick = initial_state.tick_num;
        self.total_height = 0.;
    }

    fn get_reward(&mut self, player: &PlayerData, state: &GameState, previous_action: &[f32]) -> f32 {
        if self.curr_tick != state.tick_num && player.ball_touched {
            self.total_height += state.ball.position.z;
            self.num_ticks_touched += 1;
            self.curr_tick = state.tick_num;
        }

        // Trying to make the ratio when ball.pos.z == self.target_height equal to 1.0 .
        // Adjusting the divisor of target_height (currently 4.) adjusts the slope the ratio. Example of target_height = 400: 400/4 = 100 -> 400+100 (where 100 == ball.pos.z - self.target_height) = ratio of 2.0, and then .powf(1.25) .
        // Doing powf(1.25) is optional and may or may not help
        let mut height_ratio = (state.ball.position.z - self.target_height + (self.target_height / 4.)) / (self.target_height / 4.).powf(1.25);

        if height_ratio < self.min_height_ratio {
            height_ratio = self.min_height_ratio;
        } else if height_ratio > self.max_height_ratio {
            height_ratio = self.max_height_ratio;
        }

        self.reward_fn.get_reward(player, state, previous_action) * height_ratio
    }

    fn get_final_reward(&mut self, player: &PlayerData, state: &GameState, previous_action: &[f32]) -> f32 {
        self.get_reward(player, state, previous_action)
    }
}
