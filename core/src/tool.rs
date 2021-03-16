//! Traits for abstraction of various engine tools (asset importers).

/// Trait that represents a functionality of a useful tool (eg. img2bf, bfinfo).
///
/// # Execution
///
/// Every tool can be executed with a specified set of parameters. Some tools
/// do side-effects according to the parameters (eg. creating a file on a disk
/// drive) but others may compute something and return it.
///
/// # Auto-complete
///
/// In some cases it may be possible to run the tool with multiple parameter
/// configurations. When the different possible parameters depend on the input
/// file it may be feasible to automatically generate the possible options
/// and let the user choose from the options instead of requiring to manually
/// fill all the parameters.
///
/// The `auto_complete()` function does exactly this. In accepts the parameters
/// struct that is supposed to be partially filled and computes all possible
/// fully filled parameters structs.
///
/// For example mesh imported may auto-complete all the different meshes in an
/// imported file.
///
pub trait Tool {
    /// A type that represents possible parameters accepted by this tool. This
    /// can be a struct that is used with library such as `structopt` to automatically
    /// generate a command line parser.
    type Params;

    /// Optional type that represents output of the `execute` function.
    type Result;

    /// Performs the effect of this tool with specified parameters.
    fn execute(&self, params: Self::Params) -> Self::Result;

    /// Computes all possible fully specified parameter options from the
    /// provided partially specified parameters.
    fn auto_complete(&self, params: Self::Params) -> Vec<Self::Params> {
        vec![params]
    }
}
