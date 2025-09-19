pub mod home;
pub mod auth;
pub mod laws;
pub mod proposals;
pub mod voting;
pub mod profile;

pub use home::HomePage;
pub use auth::{LoginPage, RegisterPage};
pub use laws::LawsPage;
pub use proposals::ProposalsPage;
pub use voting::VotingPage;
pub use profile::ProfilePage;
