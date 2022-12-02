use crate::nested::{NavigationContext, Request};
use crate::switch::SwitchContext;
use crate::target::Target;
use gloo_history::{AnyHistory, BrowserHistory, History, HistoryListener, Location};
use std::fmt::Debug;
use std::marker::PhantomData;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RouterContext<S: Target> {
    _marker: PhantomData<S>,
    history: AnyHistory,
}

impl<T> RouterContext<T> where T: Target {}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct RouterProps {
    pub children: Children,
}

#[derive(Debug)]
pub enum Msg {
    RouteChanged(Location),
    ChangeRoute(Request),
}

pub struct Router<T: Target> {
    history: AnyHistory,
    _listener: HistoryListener,
    context: RouterContext<T>,
    target: Option<T>,
}

impl<S> Component for Router<S>
where
    S: Target + 'static,
{
    type Message = Msg;
    type Properties = RouterProps;

    fn create(ctx: &Context<Self>) -> Self {
        let history = AnyHistory::Browser(BrowserHistory::new());

        let context = RouterContext {
            _marker: Default::default(),
            history: history.clone(),
        };

        let cb = ctx.link().callback(Msg::RouteChanged);

        let target = Self::parse_location(history.location());

        let listener = {
            let history = history.clone();
            history.clone().listen(move || {
                cb.emit(history.location());
            })
        };

        Self {
            history,
            _listener: listener,
            context,
            target,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        log::debug!("update: {msg:?}");

        match msg {
            Msg::RouteChanged(location) => {
                let target = Self::parse_location(location);
                if target != self.target {
                    self.target = target;
                    return true;
                }
            }
            Msg::ChangeRoute(request) => {
                log::debug!("Pushing state: {:?}", request.path);
                let route = format!("/{}", request.path.join("/"));
                self.history.push(route);
            }
        }

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let context = self.context.clone();

        let base = self
            .target
            .as_ref()
            .map(|t| t.render_path())
            .unwrap_or_default();

        let navigation = NavigationContext {
            base,
            parent: ctx.link().callback(Msg::ChangeRoute),
        };

        let switch = SwitchContext {
            navigation: navigation.clone(),
            target: self.target.clone(),
        };

        html! (
            <ContextProvider<RouterContext<S>> {context}>
                <ContextProvider<NavigationContext> context={navigation}>
                    <ContextProvider<SwitchContext<S>> context={switch}>
                        { for ctx.props().children.iter() }
                    </ContextProvider<SwitchContext<S>>>
                </ContextProvider<NavigationContext>>
            </ContextProvider<RouterContext<S>>>
        )
    }
}

impl<T: Target> Router<T> {
    fn parse_location(location: Location) -> Option<T> {
        let path: Vec<&str> = location.path().split('/').skip(1).collect();
        log::debug!("Path: {path:?}");
        let target = T::parse_path(&path);
        log::debug!("New target: {target:?}");
        target
    }
}

#[hook]
pub fn use_router<S>() -> Option<RouterContext<S>>
where
    S: Target + 'static,
{
    use_context::<RouterContext<S>>()
}
