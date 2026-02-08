use super::*;
use crate::persistance::GuiConfiguration;

//Helper function to genearete a dummy config for testing
fn mock_config() -> GuiConfiguration {
    //Uses the default config empty config
    GuiConfiguration::default()
}

#[test]
fn test_app_initialization() {
    let config = mock_config();

    let (app, _task) = BBImager::new(config);

    assert_eq!(app.screen.len(), 1);
    assert!(matches!(app.screen[0], Screen::Home));

    //veirfy we start on the same screen
    assert!(app.selected_board.is_none());
    assert!(app.selected_image.is_none());
    assert!(app.selected_dst.is_none());
}

#[test]
fn test_navigation_flow() {
    let config = mock_config();
    let (mut app, _) = BBImager::new(config);

    //simulate user selectiong a board
    let _cmd = app.push_page(Screen::ExtraConfiguration(
        pages::ConfigurationId::Customization,
    ));

    assert_eq!(app.screen.len(), 2);
    assert!(matches!(
        app.screen.last().unwrap(),
        Screen::ExtraConfiguration(pages::ConfigurationId::Customization)
    ));
}

// Helper to mock a Board
fn mock_board() -> config::Device {
    config::Device {
        name: "BeagleBone Black".to_string(),
        tags: Default::default(),
        icon: None,
        description: "A cool board".to_string(),
        flasher: Default::default(),
        documentation: None,
        instructions: None,
    }
}

#[test]
fn test_board_selection() {
    let config = mock_config();
    let (mut app, _) = BBImager::new(config);

    // 1. Inject Mock Data
    // Construct a full Config object containing our mock board
    let board = mock_board();
    let injection_config = bb_config::Config {
        imager: bb_config::config::Imager {
            devices: vec![board],
            ..Default::default()
        },
        ..Default::default()
    };
    // Merge it into the app state (accessing private field 'boards')
    app.boards.merge(injection_config);

    // 2. Simulate User Action (Select the first board, index 0)
    // We use the application's update logic directly
    let _cmd = message::update(&mut app, BBImagerMessage::SelectBoard(0));

    // 3. Assert State
    assert!(app.selected_board.is_some());
    assert_eq!(app.selected_board.unwrap(), 0);

    // Verify side effects: Is the destination cleared?
    assert!(app.selected_dst.is_none());
}
