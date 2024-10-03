use std::sync::Arc;

use iced::{
    futures::StreamExt,
    widget::{column, container, row, scrollable, stack},
    Element, Length, Task,
};
use nip_55::nip_46::{Nip46OverNip55ServerStream, Nip46RequestApproval};
use nostr_sdk::PublicKey;

use crate::{
    db::Database,
    fedimint::{Wallet, WalletView},
    nostr::{NostrModuleMessage, NostrState},
    routes::{self, bitcoin_wallet, unlock, Loadable, Route, RouteName},
    ui_components::{sidebar, Toast, ToastManager, ToastStatus},
};

#[derive(Debug, Clone)]
pub enum Message {
    Routes(routes::Message),

    DbDeleteAllData,

    UpdateWalletView(WalletView),

    NostrModule(NostrModuleMessage),
    UpdateNostrState(NostrState),

    CopyStringToClipboard(String),

    IncomingNip46Request(
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    ),
    ApproveFirstIncomingNip46Request,
    RejectFirstIncomingNip46Request,

    AddToast(Toast),
    CloseToast(usize),
}

pub struct App {
    pub page: Route,
    toasts: Vec<Toast>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            page: Route::new_locked(),
            toasts: Vec::new(),
        }
    }
}

impl App {
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Routes(routes_msg) => self.page.update(routes_msg),
            Message::DbDeleteAllData => {
                if let Route::Unlock(unlock::Page {
                    db_already_exists, ..
                }) = &mut self.page
                {
                    Database::delete();
                    *db_already_exists = false;
                }

                Task::none()
            }
            Message::UpdateWalletView(wallet_view) => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.loadable_wallet_view = Loadable::Loaded(wallet_view.clone());
                }

                if let Route::BitcoinWallet(bitcoin_wallet) = &mut self.page {
                    bitcoin_wallet.update(bitcoin_wallet::Message::UpdateWalletView(wallet_view))
                } else {
                    Task::none()
                }
            }
            Message::NostrModule(nostr_module_message) => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.nostr_module.update(nostr_module_message);
                }

                Task::none()
            }
            Message::UpdateNostrState(nostr_state) => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.nostr_state = nostr_state;
                }

                Task::none()
            }
            Message::CopyStringToClipboard(text) => {
                match arboard::Clipboard::new().map(|mut clipboard| clipboard.set_text(text)) {
                    Ok(_) => Task::done(Message::AddToast(Toast {
                        title: "Copied to clipboard".to_string(),
                        body: "The text has been copied to your clipboard.".to_string(),
                        status: ToastStatus::Good,
                    })),
                    Err(e) => Task::done(Message::AddToast(Toast {
                        title: "Failed to copy to clipboard".to_string(),
                        body: e.to_string(),
                        status: ToastStatus::Bad,
                    })),
                }
            }
            Message::IncomingNip46Request(data) => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.in_flight_nip46_requests.push_back(data);
                }

                Task::none()
            }
            Message::ApproveFirstIncomingNip46Request => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Approve).unwrap();
                    }
                }

                Task::none()
            }
            Message::RejectFirstIncomingNip46Request => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Reject).unwrap();
                    }
                }

                Task::none()
            }
            Message::AddToast(toast) => {
                self.toasts.push(toast);

                Task::none()
            }
            Message::CloseToast(index) => {
                self.toasts.remove(index);

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let Self { page, .. } = self;

        let mut content: Element<Message> = Element::new(scrollable(
            container(column![page.view()].spacing(20).padding(20)).center_x(Length::Fill),
        ));

        if page.to_name() != RouteName::Unlock {
            content = Element::new(row![sidebar(self), content]);
        };

        let content: Element<_, _, _> = container(content).center_y(Length::Fill).into();
        let toast_manager: Element<_, _, _> =
            ToastManager::new(&self.toasts, Message::CloseToast).into();

        stack![content, toast_manager].into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let Some(connected_state) = self.page.get_connected_state() else {
            return iced::Subscription::none();
        };

        let wallet = connected_state.wallet.clone();

        let db = connected_state.db.clone();

        let wallet_sub = iced::Subscription::run_with_id(
            std::any::TypeId::of::<Wallet>(),
            // We're wrapping `stream` in a `stream!` macro to make it lazy (meaning `stream` isn't
            // created unless the outer `stream!` is actually used). This is necessary because the
            // outer `stream!` is created on every update, but will only be polled if the subscription
            // ID is new.
            async_stream::stream! {
                let mut stream = wallet.get_update_stream().map(Message::UpdateWalletView);

                while let Some(msg) = stream.next().await {
                    yield msg;
                }
            },
        );

        let nip46_sub = iced::Subscription::run_with_id(
            std::any::TypeId::of::<Nip46OverNip55ServerStream>(),
            // We're wrapping `stream` in a `stream!` macro to make it lazy (meaning `stream` isn't
            // created unless the outer `stream!` is actually used). This is necessary because the
            // outer `stream!` is created on every update, but will only be polled if the subscription
            // ID is new.
            async_stream::stream! {
                let mut stream = Nip46OverNip55ServerStream::start("/tmp/nip55-kind24133.sock", db)
                    .unwrap()
                    .map(|(request_list, public_key, response_sender)| {
                        Message::IncomingNip46Request(Arc::new((
                            request_list,
                            public_key,
                            response_sender,
                        )))
                    });

                while let Some(msg) = stream.next().await {
                    yield msg;
                }
            },
        );

        let nostr_sub = connected_state
            .nostr_module
            .subscription()
            .map(Message::UpdateNostrState);

        iced::Subscription::batch(vec![nip46_sub, wallet_sub, nostr_sub])
    }
}
