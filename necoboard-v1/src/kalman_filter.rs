#[derive(Debug, Clone)]
pub struct KalmanFilter {
    state: Option<Gaussian>,
}

impl KalmanFilter {
    const STATE_SIGMA: f32 = 2.0;
    const NOISE_SIGMA: f32 = 10.0;

    pub fn new() -> KalmanFilter {
        KalmanFilter { state: None }
    }

    pub fn predict(&mut self, observation: f32) -> f32 {
        // Kalman filter
        // TODO: チャタリング対策

        if let Some(ref mut state) = self.state {
            let prior = Gaussian::new(state.mu, state.sigma + Self::NOISE_SIGMA);
            let gain = prior.sigma / (prior.sigma + Self::STATE_SIGMA);
            *state = Gaussian::new(
                prior.mu + gain * (observation - prior.mu),
                (1.0 - gain) * prior.sigma,
            );
            return state.mu;
        }

        self.state = Some(Gaussian::new(observation, Self::STATE_SIGMA));
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
