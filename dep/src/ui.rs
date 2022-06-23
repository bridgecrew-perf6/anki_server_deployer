use std::default;

use crate::{
    error::{ApplicationError, CMDError, LoadError},
    logic::{
        add_startmenu, addusr, async_read_to_string, chgepass, delusr, deploy_count_path,
        import_to_sysstore, install_servcerts, lookup_ip, lsusr, pc_ver_required, pcip_modify,
        read_parse_deploy_count, root_dir, send_shortcut,
    },
};
use iced::{
    alignment, button, executor, futures::FutureExt, text_input, Application, Button, Checkbox,
    Color, Column, Command, Container, Element, Length, Radio, Row, Settings, Space, Subscription,
    Text, TextInput, Toggler,
};
use iced_native::{
    futures::{self, channel::mpsc, StreamExt},
    subscription,
};
pub enum App {
    Loading,
    Loaded(State),
}
// merge Step and Message
#[derive(Debug)]
pub struct State {
    verok: bool,
    deploy_count: u8,
    back_button: button::State,
    next_button: button::State,
    current: u8,
    ///welcome
    username: String,
    password: String,
    state_user: text_input::State,
    state_pass: text_input::State,
    selection: Option<UserOperation>,
    list_radio_enable: bool,

    submit_button: button::State,
    useracnt_optips: String,

    // Set ip
    pcip_modified: bool,
    server_certs_install: bool,
    rootca_to_sysstore: bool,
    ///checkbox
    send_rootca_to_phone: bool,
    ipaddr: String,
    state_sync: text_input::State,
    state_media: text_input::State,
    ready_for_pcmod: bool,
    ready_for_certsin: bool,
    ready_for_sysstore: bool,
    ready_for_lookup: bool,

    // shortcut to desktop
    shortcut_sent: bool,
    search_enable: bool,
    ready_for_shortcut: bool,
    ready_for_search: bool,
}
impl Default for State {
    fn default() -> Self {
        State {
            verok: false,
            deploy_count: 0,
            back_button: button::State::new(),
            next_button: button::State::new(),
            current: 0,

            useracnt_optips: String::new(),

            username: String::new(),
            password: String::new(),
            state_pass: text_input::State::new(),
            state_user: text_input::State::new(),
            selection: None,
            list_radio_enable: false,
            submit_button: button::State::new(),

            pcip_modified: false,
            server_certs_install: false,
            rootca_to_sysstore: false,
            send_rootca_to_phone: false,
            ipaddr: String::new(),
            state_sync: text_input::State::new(),
            state_media: text_input::State::new(),
            ready_for_pcmod: false,
            ready_for_certsin: false,
            ready_for_sysstore: false,
            ready_for_lookup: false,

            shortcut_sent: false,
            search_enable: false,
            ready_for_shortcut: false,
            ready_for_search: false,
        }
    }
}
impl State {
    fn advance(&mut self) {
        if self.can_continue() {
            self.current += 1;
        }
    }

    fn go_back(&mut self) {
        if self.has_previous() {
            self.current -= 1;
        }
    }

    fn has_previous(&self) -> bool {
        self.current > 0
    }
    /// add step constrained conditions
    fn can_continue(&self) -> bool {
        self.current + 1 < 4 && true
    }
}
#[derive(Debug)]
struct Steps {
    steps: Vec<Step>,
    current: usize,
}
impl Default for Steps {
    fn default() -> Self {
        Steps {
            steps: vec![
                Step::Welcome {
                    username: String::new(),
                    password: String::new(),
                    state_pass: text_input::State::new(),
                    state_user: text_input::State::new(),
                    selection: None,
                    list_radio: false,
                },
                Step::SetIP,
                Step::Shortcut,
                Step::User,
            ],
            current: 0,
        }
    }
}
impl Steps {
    // fn new() -> Steps {
    //     Steps {
    //         steps: vec![Step::Welcome, Step::SetIP, Step::Shortcut, Step::User],
    //         current: 0,
    //     }
    // }
    fn title(&self) -> &str {
        self.steps[self.current].title()
    }
    // fn subscription(&self) {
    //     self.steps[self.current].subscription();
    // }
    // fn update(&mut self, msg: Message) {
    //     self.steps[self.current].update(msg);
    // }

    fn advance(&mut self) {
        if self.can_continue() {
            self.current += 1;
        }
    }

    fn go_back(&mut self) {
        if self.has_previous() {
            self.current -= 1;
        }
    }

    fn has_previous(&self) -> bool {
        self.current > 0
    }

    fn can_continue(&self) -> bool {
        self.current + 1 < self.steps.len() && self.steps[self.current].can_continue()
    }
    fn view(&mut self, deploy_count: u8, verok: bool) -> Element<Message> {
        self.steps[self.current].view(deploy_count, verok)
    }
}
#[derive(Debug, Clone)]
enum Step {
    Welcome {
        username: String,
        password: String,
        state_user: text_input::State,
        state_pass: text_input::State,
        selection: Option<UserOperation>,
        list_radio: bool,
    },
    SetIP,
    Shortcut,
    User,
}
impl Default for Step {
    fn default() -> Self {
        Step::Welcome {
            username: String::new(),
            password: String::new(),
            state_pass: text_input::State::new(),
            state_user: text_input::State::new(),
            selection: None,
            list_radio: false,
        }
    }
}
impl<'a> Step {
    fn title(&self) -> &str {
        match self {
            Step::Welcome { .. } => "欢迎",
            Self::SetIP => "设置IP",
            Self::Shortcut => "快捷方式",
            Self::User => "账户管理",
        }
    }

    fn has_previous(&self, current: u8) -> bool {
        current > 0
    }

    /// user management widgets for Welcome and user manage pages
    fn radio(
        selection: Option<UserOperation>,
        username: &str,
        password: &str,
        state_user: &'a mut text_input::State,
        state_pass: &'a mut text_input::State,
    ) -> Column<'a, Message> {
        // radio section
        let radio_selections = Column::new()
            .padding(20)
            .spacing(10)
            .push(Text::new("用户管理").size(24))
            .push(UserOperation::all().iter().cloned().fold(
                Row::new().padding(10).spacing(20),
                |choices, language| {
                    choices.push(Radio::new(
                        language,
                        language,
                        selection,
                        Message::UserOperationSeleted,
                    ))
                },
            ));
        // submit button for add/del/pass
        // textinput/output section
        let textio_section = match selection {
            None => Column::new().push(Text::new("没有进行任何账号相关操作")),
            Some(UserOperation::Add) => {
                // two text input bars which denote username and password respectively
                let user_input = TextInput::new(
                    state_user,
                    "Type something to continue...",
                    username,
                    Message::UserInputChanged,
                )
                .padding(10)
                .size(30);
                let pass_input = TextInput::new(
                    state_pass,
                    "Type something to continue...",
                    password,
                    Message::PassInputChanged,
                )
                .padding(10)
                .size(30);
                Column::new().push(user_input).push(pass_input)
            }
            Some(UserOperation::List) => {
                // one text input which has read-only permission and place usernames

                Column::new().push(Text::new(format!("users: {}", username)))
                // Column::new().push(Text::new(format!("users: {}", userlist.users)))
            }
            Some(UserOperation::Delete) => {
                // one text input which has write permission and for users to be deleted
                let user_input = TextInput::new(
                    state_user,
                    "Type something to continue...",
                    username,
                    Message::UserInputChanged,
                )
                .padding(10)
                .size(30);
                Column::new().push(user_input)
            }
            Some(UserOperation::Pass) => {
                // two text input bars which denote username and new password respectively
                let user_input = TextInput::new(
                    state_user,
                    "Type something to continue...",
                    username,
                    Message::UserInputChanged,
                )
                .padding(10)
                .size(30);
                let newpass_input = TextInput::new(
                    state_pass,
                    "Type something to continue...",
                    password,
                    Message::PassInputChanged,
                )
                .padding(10)
                .size(30);
                Column::new().push(user_input).push(newpass_input)
            }
        };

        // put radio and text sections in a new container
        radio_selections.push(textio_section)
    }
    fn container(title: &str) -> Column<'a, Message> {
        Column::new().spacing(20).push(Text::new(title).size(50))
    }
    fn container_without_title() -> Column<'a, Message> {
        Column::new().spacing(20)
    }
    /// display user manage controls if deploy_count >=1
    /// render text with red if verok is false,else green
    fn welcome(
        deploy_count: u8,
        verok: bool,
        selection: Option<UserOperation>,
        username: &str,
        password: &str,
        state_user: &'a mut text_input::State,
        state_pass: &'a mut text_input::State,
    ) -> Column<'a, Message> {
        if verok {
            if deploy_count >= 1 {
                Self::container_without_title()
                    .push(Text::new("PC Anki 版本符合要求！"))
                    .push(Self::radio(
                        selection, username, password, state_user, state_pass,
                    ))
            } else {
                Self::container_without_title()
                    .push(Text::new("PC Anki 版本符合要求！").color(Color::from_rgb8(0, 255, 0)))
            }
        } else {
            Self::container_without_title().push(
                Text::new("PC Anki 版本不符合要求！请安装符合要求的Anki再打开本程序")
                    .color(Color::from_rgb8(255, 0, 0)),
            )
        }
    }
    fn test() -> Column<'a, Message> {
        Self::container("Welcome!").push(Text::new(
            "This is a simple tour meant to showcase a bunch of widgets \
                 that can be easily implemented on top of Iced.",
        ))
    }
    fn view(&mut self, deploy_count: u8, verok: bool) -> Element<Message> {
        match self {
            Step::Welcome {
                username,
                password,
                state_user,
                state_pass,
                selection,
                list_radio,
            } => Self::welcome(
                deploy_count,
                verok,
                *selection,
                username,
                password,
                state_user,
                state_pass,
            ),
            Self::SetIP => Self::test(),
            Self::Shortcut => Self::test(),
            Self::User => Self::test(),
        }
        .into()
    }

    fn can_continue(&self) -> bool {
        match self {
            Self::Welcome { .. } => true,
            Self::SetIP => true,
            Self::Shortcut => true,
            Self::User => false,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    BackPressed,
    NextPressed,
    Loaded(Result<LoadingConf, LoadError>),
    ExternalCMD(Result<(), CMDError>),

    UserOperationSeleted(UserOperation),
    UserInputChanged(String),
    PassInputChanged(String),
    TextContentChanged(Event),
    /// add/del/pass
    Submit(String, String),

    SyncAddr(String),
    TogglerChanged(bool),
    IPAddrChanged(Event),
    PCIPChanged(Event),
    CAImported(Event),
    ServCertsInstalled(Event),

    MSLNKSent(Event),
    SearchEnabled(Event),
}
#[derive(Debug, Clone)]
pub enum Event {
    Start,
    Received(Option<String>),
}
fn button<'a, Message: Clone>(state: &'a mut button::State, label: &str) -> Button<'a, Message> {
    Button::new(
        state,
        Text::new(label).horizontal_alignment(alignment::Horizontal::Center),
    )
    .padding(12)
    .min_width(100)
}
/// print tips if file path which is relative to current executable path not exist
fn loading_message<'a>() -> Element<'a, Message> {
    let controls = Column::new();
    let content = if root_dir().exists() {
        controls.push(
            Text::new("Loading...")
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(50),
        )
    } else {
        controls
            .push(
                Text::new("Loading...")
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .size(50),
            )
            .push(
                Text::new("未找到对应的文件夹")
                    .color(Color::from_rgb8(255, 0, 0))
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
    };
    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}
impl<'a> App {
    fn title(current: u8) -> &'static str {
        match current {
            0 => "欢迎",
            1 => "设置IP",
            2 => "快捷方式",
            3 => "账户管理",
            _ => "",
        }
    }
    fn has_previous(current: u8) -> bool {
        current > 0
    }
    /// add step constrained conditions
    fn can_continue(current: u8, send_rootca_to_phone: bool) -> bool {
        // set ip page
        let can_continue = {
            if send_rootca_to_phone {
                true
            } else {
                false
            }
        };
        if current == 1 {
            return current + 1 < 4 && can_continue;
        }

        current + 1 < 4
    }
    fn user_manage(
        selection: Option<UserOperation>,
        username: &str,
        password: &str,
        state_user: &'a mut text_input::State,
        state_pass: &'a mut text_input::State,
        submit_btn_state: &'a mut button::State,
        useracnt_optips: &str,
    ) -> Column<'a, Message> {
        Self::container_without_title()
            .align_items(alignment::Alignment::Center)
            .push(Self::radio(
                selection,
                username,
                password,
                state_user,
                state_pass,
                submit_btn_state,
                useracnt_optips,
            ))
    }

    fn shortcut_search(shortcut_sent: bool, search_enable: bool) -> Column<'a, Message> {
        let (shortcut_status, shortcut_clr) = if shortcut_sent {
            ("OK", Color::from_rgb8(0, 255, 0))
        } else {
            ("...", Color::from_rgb8(255, 0, 0))
        };
        let (search_status, search_clr) = if search_enable {
            ("OK", Color::from_rgb8(0, 255, 0))
        } else {
            ("...", Color::from_rgb8(255, 0, 0))
        };

        let shortcut_section = Row::new()
            .push(
                Text::new("发送anki_server到桌面：")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new(shortcut_status)
                    .color(shortcut_clr)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        let seach_section = Row::new()
            .push(
                Text::new("复制快捷方式到Windows开始菜单：")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new(search_status)
                    .color(search_clr)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        Self::container_without_title()
            .align_items(alignment::Alignment::Center)
            .push(shortcut_section)
            .push(seach_section)
            .push(
                Text::new("这意味着可以从开始菜单搜索启动服务器软件")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
    }
    fn set_ip(
        pcip_modified: bool,
        server_certs_install: bool,
        rootca_to_sysstore: bool,
        send_rootca_to_phone: bool,
        ipaddr: &str,
        state_sync: &'a mut text_input::State,
        state_media: &'a mut text_input::State,
    ) -> Column<'a, Message> {
        // state_check section text
        let (pcip_mod_status, pcip_mod_clr) = if pcip_modified {
            ("OK", Color::from_rgb8(0, 255, 0))
        } else {
            ("...", Color::from_rgb8(255, 0, 0))
        };
        let pcip_mod = Row::new()
            .push(
                Text::new("修改PC Anki 同步地址：")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new(pcip_mod_status)
                    .color(pcip_mod_clr)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        // rootca_to_sysstore
        let (sysstore_status, sysstore_clr) = if let true = rootca_to_sysstore {
            ("OK", Color::from_rgb8(0, 255, 0))
        } else {
            ("...", Color::from_rgb8(255, 0, 0))
        };
        let sysstore = Row::new()
            .push(
                Text::new("安装证书到系统证书区：")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new(sysstore_status)
                    .color(sysstore_clr)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        // server_certs_install
        let (certs_install_status, certs_install_clr) = if let true = server_certs_install {
            ("OK", Color::from_rgb8(0, 255, 0))
        } else {
            ("...", Color::from_rgb8(255, 0, 0))
        };
        let certs_install = Row::new()
            .push(
                Text::new("签发服务器证书并安装：")
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new(certs_install_status)
                    .color(certs_install_clr)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        let state_check_section = Self::container_without_title()
            .push(pcip_mod)
            .push(sysstore)
            .push(certs_install);

        // sync and media sync address display section text_input
        let sync_addr = Row::new()
            .push(Text::new("同步地址：").vertical_alignment(alignment::Vertical::Bottom))
            .push(
                TextInput::new(
                    state_sync,
                    "...",
                    &format!("https://{}:27701", ipaddr),
                    Message::SyncAddr,
                )
                .padding(10)
                .size(20),
            )
            .align_items(alignment::Alignment::Fill);
        let media_addr = Row::new()
            .push(Text::new("媒体文件同步地址：").vertical_alignment(alignment::Vertical::Bottom))
            .push(
                TextInput::new(
                    state_media,
                    "...",
                    &format!("https://{}:27701/msync", ipaddr),
                    Message::SyncAddr,
                )
                .padding(10)
                .size(20),
            )
            .align_items(alignment::Alignment::Fill);
        let addr_show_section = Self::container_without_title()
            .push(sync_addr)
            .push(media_addr);

        // user confirm checkbox to make next step can continue
        let next_confirm = Self::container_without_title().push(
            Container::new(Toggler::new(
                send_rootca_to_phone,
                String::from("将rootCA.crt发送到手机并安装，修改手机Anki同步地址，确认？"),
                Message::TogglerChanged,
            ))
            .padding([0, 40])
            .padding(10),
        );
        Self::container_without_title()
            .align_items(alignment::Alignment::Center)
            .push(state_check_section)
            .push(addr_show_section)
            .push(next_confirm)
    }
    /// user management widgets for Welcome and user manage pages
    fn radio(
        selection: Option<UserOperation>,
        username: &str,
        password: &str,
        state_user: &'a mut text_input::State,
        state_pass: &'a mut text_input::State,
        submit_btn_state: &'a mut button::State,
        useracnt_optips: &str,
    ) -> Column<'a, Message> {
        // radio section
        let radio_selections = Column::new()
            .padding(20)
            .spacing(10)
            .align_items(alignment::Alignment::Center)
            .push(
                Text::new("用户管理")
                    .size(24)
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(UserOperation::all().iter().cloned().fold(
                Row::new().padding(10).spacing(20),
                |choices, language| {
                    choices.push(Radio::new(
                        language,
                        language,
                        selection,
                        Message::UserOperationSeleted,
                    ))
                },
            ));

        // submit button for add/del/pass
        // textinput/output section
        let user_input = TextInput::new(state_user, "用户名", username, Message::UserInputChanged)
            .padding(10)
            .size(30);
        let pass_input = TextInput::new(state_pass, "密码", password, Message::PassInputChanged)
            .padding(10)
            .size(30);
        let textio_section = match selection {
            None => Column::new().push(Text::new("没有进行任何账号相关操作")),
            Some(UserOperation::List) => {
                // one text input which has read-only permission and place usernames

                Column::new().push(Text::new(format!("users: {}", username)))
                // Column::new().push(Text::new(format!("users: {}", userlist.users)))
            }
            Some(UserOperation::Delete) => {
                // one text input which has write permission and for users to be deleted

                Column::new().push(user_input)
            }
            _ => {
                // add and pass
                // two text input bars which denote username and new password respectively
                Column::new().spacing(10).push(user_input).push(pass_input)
            }
        };

        // submit button section
        let submit_btn_section = match selection {
            None | Some(UserOperation::List) => Row::new().push(Text::new("")),
            _ => {
                let btn = Button::new(submit_btn_state, Text::new("提交"))
                    .on_press(Message::Submit(username.into(), password.into()))
                    .style(style::Button::Primary);

                Row::new().push(btn).push(
                    Text::new(format!("{}", useracnt_optips)).color(Color::from_rgb8(255, 0, 0)),
                )
            }
        };

        // put radio and text sections in a new container
        radio_selections
            .push(textio_section)
            .push(submit_btn_section)
    }
    fn container(title: &str) -> Column<'a, Message> {
        Column::new().spacing(20).push(Text::new(title).size(50))
    }
    fn container_without_title() -> Column<'a, Message> {
        Column::new().spacing(20)
    }
    /// display user manage controls if deploy_count >=1
    /// render text with red if verok is false,else green
    fn welcome(
        deploy_count: u8,
        verok: bool,
        selection: Option<UserOperation>,
        username: &str,
        password: &str,
        state_user: &'a mut text_input::State,
        state_pass: &'a mut text_input::State,
        submit_btn_state: &'a mut button::State,
        useracnt_optips: &str,
    ) -> Column<'a, Message> {
        if verok {
            if deploy_count >= 1 {
                Self::container_without_title()
                    .align_items(alignment::Alignment::Center)
                    .push(Text::new("PC Anki 版本符合要求！").color(Color::from_rgb8(0, 255, 0)))
                    .push(Self::radio(
                        selection,
                        username,
                        password,
                        state_user,
                        state_pass,
                        submit_btn_state,
                        useracnt_optips,
                    ))
            } else {
                Self::container_without_title()
                    .align_items(alignment::Alignment::Center)
                    .push(Text::new("PC Anki 版本符合要求！").color(Color::from_rgb8(0, 255, 0)))
            }
        } else {
            Self::container_without_title()
                .align_items(alignment::Alignment::Center)
                .push(
                    Text::new("PC Anki 版本不符合要求！请安装符合要求的Anki再打开本程序")
                        .color(Color::from_rgb8(255, 0, 0)),
                )
        }
    }
    fn test() -> Column<'a, Message> {
        Self::container("Welcome!").push(Text::new(
            "This is a simple tour meant to showcase a bunch of widgets \
                 that can be easily implemented on top of Iced.",
        ))
    }
}
impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    ///read file deploy_count.txt when app is initiated
    fn new(_flags: ()) -> (App, Command<Message>) {
        (
            App::Loading,
            Command::perform(LoadingConf::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        let title = match self {
            App::Loading => "loading",
            App::Loaded(state) => Self::title(state.current),
        };
        format!("{} - anki sync server deployer", title)
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            App::Loaded(state) => {
                if state.list_radio_enable {
                    return list().map(Message::TextContentChanged);
                }

                // set ip page
                if state.ready_for_lookup {
                    // look up ip address

                    return look_up().map(Message::IPAddrChanged);
                }
                // set pc anki sync address
                if state.ready_for_pcmod {
                    return pcip_mod(&state.ipaddr).map(Message::PCIPChanged);
                }
                // install rootCA and export it to sys store
                if state.ready_for_sysstore {
                    return import_sysstore().map(Message::CAImported);
                }
                //  gen server certs and install(write their paths to Stetings.toml)
                if state.ready_for_certsin {
                    return srvcerts_install(&state.ipaddr).map(Message::ServCertsInstalled);
                }

                // step 3 send shortcui to desktop
                if state.ready_for_shortcut {
                    return shortcut().map(Message::MSLNKSent);
                }
                if state.ready_for_search {
                    return seach().map(Message::SearchEnabled);
                }

                Subscription::none()
            }
            _ => Subscription::none(),
        }
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            App::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = App::Loaded(State {
                            verok: state.verok,
                            deploy_count: state.deploy_count,
                            ..State::default()
                        });
                    }
                    _ => {}
                }
                Command::none()
            }
            App::Loaded(state) => {
                match message {
                    Message::BackPressed => {
                        state.go_back();
                        // step 2 set ip page
                        if state.current == 1 {
                            state.ready_for_lookup = true;
                        } else if state.current == 2 {
                            // step 3  send shortcut to desktop
                            state.ready_for_shortcut = true;
                            state.shortcut_sent = false;
                            state.search_enable = false;
                            state.list_radio_enable = false;
                        }
                        Command::none()
                    }
                    Message::NextPressed => {
                        state.advance();
                        // step 2 set ip page
                        if state.current == 1 {
                            state.ready_for_lookup = true;
                            state.list_radio_enable = false;
                        } else if state.current == 2 {
                            // step 3  send shortcut to desktop
                            state.ready_for_shortcut = true;

                            state.shortcut_sent = false;
                            state.search_enable = false;
                        } else if state.current == 3 {
                            state.ready_for_shortcut = false;
                            state.ready_for_search = false;
                        }
                        Command::none()
                    }
                    Message::UserOperationSeleted(userop) => {
                        //    make list_radio_enable true when switched to List,ready for
                        //scription
                        state.selection = Some(userop);
                        if let UserOperation::List = userop {
                            state.list_radio_enable = true;
                        } else {
                            state.list_radio_enable = false;
                            // clear fields username and password
                            state.username.clear();
                            state.password.clear();
                            state.useracnt_optips.clear();
                        }
                        Command::none()
                    }
                    Message::UserInputChanged(new_value) => {
                        state.username = new_value;
                        // user manage page also need
                        Command::none()
                    }
                    Message::PassInputChanged(new_value) => {
                        state.password = new_value;

                        // user manage page also need
                        Command::none()
                    }
                    Message::TextContentChanged(e) => match e {
                        Event::Received(m) => {
                            if let Some(UserOperation::List) = state.selection {
                                if let Some(users) = m {
                                    state.username = users;
                                }
                            }
                            Command::none()
                        }
                        _ => Command::none(),
                    },
                    Message::IPAddrChanged(e) => match e {
                        Event::Received(m) => {
                            if let Some(addr) = m {
                                state.ipaddr = addr;
                            }
                            state.ready_for_lookup = false;
                            state.ready_for_pcmod = true;

                            // in case step back from step 3 or step one,status not update
                            state.pcip_modified = false;
                            state.rootca_to_sysstore = false;
                            state.server_certs_install = false;
                            Command::none()
                        }
                        _ => Command::none(),
                    },
                    Message::PCIPChanged(_) => {
                        state.pcip_modified = true;
                        state.ready_for_pcmod = false;
                        state.ready_for_sysstore = true;
                        Command::none()
                    }
                    Message::CAImported(_) => {
                        state.ready_for_sysstore = false;
                        state.rootca_to_sysstore = true;
                        state.ready_for_certsin = true;
                        Command::none()
                    }
                    Message::ServCertsInstalled(_) => {
                        state.ready_for_certsin = false;
                        state.server_certs_install = true;
                        Command::none()
                    }
                    Message::Submit(username, password) => {
                        // user operation tips
                        let tips = match state.selection {
                            Some(UserOperation::Delete) => {
                                if username.is_empty() {
                                    "用户名为空"
                                } else {
                                    ""
                                }
                            }
                            Some(UserOperation::List) | None => "",
                            _ => {
                                if username.is_empty() || password.is_empty() {
                                    "用户名或密码为空"
                                } else {
                                    ""
                                }
                            }
                        };
                        state.useracnt_optips = tips.into();

                        Command::perform(
                            UserAccount {
                                username: Some(username),
                                password: Some(password),
                            }
                            .userops(state.selection),
                            Message::ExternalCMD,
                        )
                    }
                    Message::ExternalCMD(_) => {
                        state.username.clear();
                        state.password.clear();
                        Command::none()
                    }
                    Message::TogglerChanged(enable) => {
                        state.send_rootca_to_phone = enable;
                        Command::none()
                    }

                    Message::MSLNKSent(_) => {
                        state.ready_for_shortcut = false;
                        state.ready_for_search = true;
                        state.shortcut_sent = true;
                        Command::none()
                    }
                    Message::SearchEnabled(_) => {
                        state.ready_for_search = false;
                        state.search_enable = true;
                        Command::none()
                    }
                    _ => Command::none(),
                }
            }
        }
    }
    fn view(&mut self) -> Element<Message> {
        match self {
            App::Loading => loading_message(),
            App::Loaded(State {
                verok,
                deploy_count,
                back_button,
                next_button,
                current,

                selection,
                username,
                password,
                state_user,
                state_pass,
                submit_button,
                useracnt_optips,

                pcip_modified,
                server_certs_install,
                rootca_to_sysstore,
                send_rootca_to_phone,
                ipaddr,
                state_sync,
                state_media,

                shortcut_sent,
                search_enable,
                ..
            }) => {
                let mut controls = Row::new();

                if Self::has_previous(*current) {
                    controls = controls.push(
                        button(back_button, "上一步")
                            .on_press(Message::BackPressed)
                            .style(style::Button::Secondary),
                    );
                }

                controls = controls.push(Space::with_width(Length::Fill));

                if Self::can_continue(*current, *send_rootca_to_phone) && *verok {
                    controls = controls.push(
                        button(next_button, "下一步")
                            .on_press(Message::NextPressed)
                            .style(style::Button::Primary),
                    );
                }
                let step_view = match current {
                    0 => Self::welcome(
                        *deploy_count,
                        *verok,
                        *selection,
                        username,
                        password,
                        state_user,
                        state_pass,
                        submit_button,
                        useracnt_optips,
                    ),
                    1 => Self::set_ip(
                        *pcip_modified,
                        *server_certs_install,
                        *rootca_to_sysstore,
                        *send_rootca_to_phone,
                        ipaddr,
                        state_sync,
                        state_media,
                    ),
                    2 => Self::shortcut_search(*shortcut_sent, *search_enable),
                    3 => Self::user_manage(
                        *selection,
                        username,
                        password,
                        state_user,
                        state_pass,
                        submit_button,
                        useracnt_optips,
                    ),
                    _ => Self::test(),
                };
                let content: Element<_> = Column::new()
                    .align_items(alignment::Alignment::Center)
                    .max_width(700)
                    // .spacing(20)
                    // .padding(20)
                    .push(step_view)
                    .push(controls)
                    .into();
                Container::new(content)
                    .height(Length::Fill)
                    .center_y()
                    .width(Length::Fill)
                    .center_x()
                    .into()
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct LoadingConf {
    verok: bool,
    deploy_count: u8,
}

impl LoadingConf {
    async fn load() -> Result<LoadingConf, LoadError> {
        let deploy_count = read_parse_deploy_count().await.unwrap();
        let pc_ver_ok = pc_ver_required();
        Ok(LoadingConf {
            verok: pc_ver_ok,
            deploy_count,
        })
    }
}
#[derive(Debug, Clone)]
pub struct UserAccount {
    username: Option<String>,
    password: Option<String>,
}
impl UserAccount {
    async fn userops(self, select: Option<UserOperation>) -> Result<(), CMDError> {
        match select {
            Some(UserOperation::Add) => self.add().await,
            Some(UserOperation::Delete) => self.del().await,
            Some(UserOperation::Pass) => self.pass().await,
            _ => Ok(()),
        }
    }
    async fn add(self) -> Result<(), CMDError> {
        if self.username.is_some() && self.password.is_some() {
            if let (Some(username), Some(password)) = (self.username, self.password) {
                addusr(username, password).unwrap();
            }
        }
        Ok(())
    }
    async fn del(self) -> Result<(), CMDError> {
        if self.username.is_some() {
            if let Some(username) = self.username {
                delusr(username).unwrap();
            }
        }
        Ok(())
    }
    async fn pass(self) -> Result<(), CMDError> {
        if self.username.is_some() && self.password.is_some() {
            if let (Some(username), Some(password)) = (self.username, self.password) {
                chgepass(username, password).unwrap();
            }
        }
        Ok(())
    }
}
enum SubscriptionState {
    Start,
    Finish,
}
enum SetIPState {
    Ready(String),
    Finish,
}
fn list() -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SubscriptionState::Start,
        move |state| list_logic(state),
    )
}
async fn list_logic(state: SubscriptionState) -> (Option<Event>, SubscriptionState) {
    match state {
        SubscriptionState::Start => {
            let users = lsusr().await.unwrap();
            (
                Some(Event::Received(Some(users))),
                SubscriptionState::Finish,
            )
        }

        SubscriptionState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}
/// look up LAN ip address
fn look_up() -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SubscriptionState::Start,
        move |state| look_up_logic(state),
    )
}
async fn look_up_logic(state: SubscriptionState) -> (Option<Event>, SubscriptionState) {
    match state {
        SubscriptionState::Start => {
            let ip = lookup_ip().unwrap();
            (Some(Event::Received(Some(ip))), SubscriptionState::Finish)
        }

        SubscriptionState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}
/// look up LAN ip address
fn pcip_mod(ipaddr: &str) -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SetIPState::Ready(ipaddr.into()),
        move |state| pcip_mod_logic(state),
    )
}
async fn pcip_mod_logic(state: SetIPState) -> (Option<Event>, SetIPState) {
    match state {
        SetIPState::Ready(ipaddr) => {
            pcip_modify(&ipaddr).await.unwrap();
            (Some(Event::Received(None)), SetIPState::Finish)
        }

        SetIPState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}
///
fn import_sysstore() -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SubscriptionState::Start,
        move |state| import_sysstore_logic(state),
    )
}
/// import rootCA to system trust store \n
/// rename .pem to .crr and send to desktop
async fn import_sysstore_logic(state: SubscriptionState) -> (Option<Event>, SubscriptionState) {
    match state {
        SubscriptionState::Start => {
            import_to_sysstore().await.unwrap();
            (Some(Event::Received(None)), SubscriptionState::Finish)
        }

        SubscriptionState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}
/// run cmd mkcert to gen server cert and key files
///
/// write their paths to file Settings.toml
fn srvcerts_install(ipaddr: &str) -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SetIPState::Ready(ipaddr.into()),
        move |state| srvcerts_install_logic(state),
    )
}
async fn srvcerts_install_logic(state: SetIPState) -> (Option<Event>, SetIPState) {
    match state {
        SetIPState::Ready(ipaddr) => {
            install_servcerts(&ipaddr).await.unwrap();
            (Some(Event::Received(None)), SetIPState::Finish)
        }

        SetIPState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}

fn shortcut() -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SubscriptionState::Start,
        move |state| shortcut_logic(state),
    )
}

async fn shortcut_logic(state: SubscriptionState) -> (Option<Event>, SubscriptionState) {
    match state {
        SubscriptionState::Start => {
            send_shortcut().await.unwrap();
            (Some(Event::Received(None)), SubscriptionState::Finish)
        }

        SubscriptionState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}

fn seach() -> Subscription<Event> {
    struct SM;
    subscription::unfold(
        std::any::TypeId::of::<SM>(),
        SubscriptionState::Start,
        move |state| seach_logic(state),
    )
}

async fn seach_logic(state: SubscriptionState) -> (Option<Event>, SubscriptionState) {
    match state {
        SubscriptionState::Start => {
            add_startmenu().await.unwrap();
            (Some(Event::Received(None)), SubscriptionState::Finish)
        }

        SubscriptionState::Finish => {
            let _: () = iced::futures::future::pending().await;
            unreachable!()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserOperation {
    Add,
    List,
    Delete,
    Pass,
}
impl UserOperation {
    fn all() -> [UserOperation; 4] {
        [
            UserOperation::Add,
            UserOperation::List,
            UserOperation::Delete,
            UserOperation::Pass,
        ]
    }
}
impl From<UserOperation> for String {
    fn from(userop: UserOperation) -> String {
        String::from(match userop {
            UserOperation::Add => "创建账号",
            UserOperation::List => "查看账号",
            UserOperation::Delete => "删除用户",
            UserOperation::Pass => "修改密码",
        })
    }
}
mod style {
    use iced::{button, Background, Color, Vector};

    pub enum Button {
        Primary,
        Secondary,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::Color(match self {
                    Button::Primary => Color::from_rgb(0.11, 0.42, 0.87),
                    Button::Secondary => Color::from_rgb(0.5, 0.5, 0.5),
                })),
                border_radius: 12.0,
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: Color::from_rgb8(0xEE, 0xEE, 0xEE),
                ..button::Style::default()
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                text_color: Color::WHITE,
                shadow_offset: Vector::new(1.0, 2.0),
                ..self.active()
            }
        }
    }
}
