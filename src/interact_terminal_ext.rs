use io_ext::InteractExt;
use io_ext_adapters::ExtInteractor;
use terminal_support::InteractTerminal;
use text_formats::TextInteractor;

pub(crate) trait InteractTerminalExt: InteractExt + InteractTerminal {}

impl<Inner: InteractTerminal> InteractTerminalExt for ExtInteractor<Inner> {}
impl<Inner: InteractTerminal + InteractExt> InteractTerminalExt for TextInteractor<Inner> {}
