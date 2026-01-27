#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) enum Screen {
    #[default]
    Home,
    BoardSelection(SearchState),
    ImageSelection(ImageSelectionState),
    DestinationSelection(SearchState),
    ExtraConfiguration(ConfigurationId),
    Flashing(FlashingState),
    FlashingConfirmation,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigurationId {
    #[default]
    Customization,
    Settings,
    About,
}

impl Screen {
    pub(crate) fn is_destination_selection(&self) -> bool {
        matches!(self, Self::DestinationSelection(_))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchState;

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct ImageSelectionState {
    flasher: bb_config::config::Flasher,
    idx: Vec<usize>,
}

impl ImageSelectionState {
    pub(crate) fn new(flasher: bb_config::config::Flasher) -> Self {
        Self {
            flasher,
            idx: Vec::new(),
        }
    }

    pub(crate) fn idx(&self) -> &[usize] {
        &self.idx
    }

    pub(crate) fn flasher(&self) -> bb_config::config::Flasher {
        self.flasher
    }

    pub(crate) fn with_added_id(mut self, id: usize) -> Self {
        self.idx.push(id);
        self
    }

    pub(crate) fn with_flasher(mut self, flasher: bb_config::config::Flasher) -> Self {
        self.flasher = flasher;
        self
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct FlashingState {
    progress: crate::helpers::ProgressBarState,
    documentation: String,
}

impl FlashingState {
    pub(crate) fn new(progress: crate::helpers::ProgressBarState, documentation: String) -> Self {
        Self {
            documentation,
            progress,
        }
    }

    pub(crate) fn update(mut self, progress: crate::helpers::ProgressBarState) -> Self {
        self.progress = progress;
        self
    }

    pub(crate) fn documentation(&self) -> &str {
        &self.documentation
    }

    pub(crate) fn progress(&self) -> &crate::helpers::ProgressBarState {
        &self.progress
    }
}
