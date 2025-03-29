#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Home,
    BoardSelection(SearchState),
    ImageSelection(ImageSelectionState),
    DestinationSelection(SearchState),
    ExtraConfiguration,
    Flashing,
    FlashingConfirmation,
}

impl Screen {
    pub(crate) fn is_destination_selection(&self) -> bool {
        matches!(self, Self::DestinationSelection(_))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchState {
    search_string: String,
}

impl SearchState {
    pub(crate) const fn new(search_string: String) -> Self {
        Self { search_string }
    }

    pub(crate) fn search_str(&self) -> &str {
        &self.search_string
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct ImageSelectionState {
    flasher: bb_config::config::Flasher,
    idx: Vec<usize>,
    search_string: String,
}

impl ImageSelectionState {
    pub(crate) fn new(flasher: bb_config::config::Flasher) -> Self {
        Self {
            flasher,
            idx: Vec::new(),
            search_string: String::new(),
        }
    }

    pub(crate) fn search_str(&self) -> &str {
        &self.search_string
    }

    pub(crate) fn idx(&self) -> &[usize] {
        &self.idx
    }

    pub(crate) fn flasher(&self) -> bb_config::config::Flasher {
        self.flasher
    }

    pub(crate) fn with_search_string(mut self, search_str: String) -> Self {
        self.search_string = search_str;
        self
    }

    pub(crate) fn with_added_id(mut self, id: usize) -> Self {
        self.idx.push(id);
        self
    }
}

#[derive(Debug, Clone)]
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
