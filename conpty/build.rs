fn main() {
    windows::build! {
        Windows::Win32::Foundation::*,
        Windows::Win32::System::Console::*,
        Windows::Win32::System::Pipes::*,
        Windows::Win32::System::Threading::*,
        Windows::Win32::System::SystemServices::*,
        Windows::Win32::System::WindowsProgramming::{INFINITE, PIPE_WAIT},
        Windows::Win32::Storage::FileSystem::*,
    };
}
