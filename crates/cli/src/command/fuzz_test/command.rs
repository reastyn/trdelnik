use clap::Subcommand;

#[derive(Subcommand)]
pub enum FuzzCommand {
    /// Run fuzz tests
    Run {
        /// Anchor project root
        #[clap()]
        test_name: String,
    },
    /// Generate fuzz tests
    New {
        /// Anchor project root
        #[clap()]
        test_name: String,
    },
}
