use core::cell::RefCell;

#[derive(Debug, Clone)]
pub struct Filter {
    state: RefCell<Option<Gaussian>>,
}

impl Filter {
    const STATE_SIGMA: f32 = 2.0;
    const NOISE_SIGMA: f32 = 10.0;

    pub fn new() -> Filter {
        Filter {
            state: RefCell::new(None),
        }
    }

    pub fn predict(&self, observation: f32) -> f32 {
        // Kalman filter
        // TODO: チャタリング対策

        if let Some(ref mut state) = *self.state.borrow_mut() {
            let prior = Gaussian::new(state.mu, state.sigma + Self::NOISE_SIGMA);
            let gain = prior.sigma / (prior.sigma + Self::STATE_SIGMA);
            *state = Gaussian::new(
                prior.mu + gain * (observation - prior.mu),
                (1.0 - gain) * prior.sigma,
            );
            return state.mu;
        }

        let mut state = self.state.borrow_mut();
        *state = Some(Gaussian::new(observation, Self::STATE_SIGMA));
        observation
    }
}

#[derive(Debug, Clone, Copy)]
struct Gaussian {
    mu: f32,
    sigma: f32,
}

impl Gaussian {
    fn new(mu: f32, sigma: f32) -> Self {
        Self { mu, sigma }
    }
}
