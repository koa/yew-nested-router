//! Routing target

use std::borrow::Cow;
use std::fmt::Debug;
use url::form_urlencoded::Serializer;
use url::Url;
use yew::Callback;

/// A target for used by a router.
pub trait Target: Clone + Debug + PartialEq + 'static {
    /// Render only our path segment.
    fn render_self(&self) -> Vec<Cow<str>> {
        let mut path = vec![];
        let mut params = vec![];
        self.render_self_into(&mut path, &mut params);
        path
    }

    /// Render the full path, including our children.
    fn render_path(&self) -> Vec<Cow<str>> {
        let mut path = vec![];
        let mut params = vec![];
        self.render_path_into(&mut path, &mut params);
        path
    }
    /// Return Err(()) if this URL is cannot-be-a-base.
    #[allow(clippy::result_unit_err)]
    fn append_url(&self, url: &mut Url) -> Result<(), ()> {
        let mut path = vec![];
        let mut params = vec![];
        self.render_path_into(&mut path, &mut params);
        // append path
        {
            let mut segments = url.path_segments_mut()?;
            segments.pop_if_empty();
            for segment in path {
                segments.push(segment.as_ref());
            }
        }

        // override query params
        let mut serializer = Serializer::new(String::new());
        for (key, value) in params {
            serializer.append_pair(key.as_ref(), value.as_ref());
        }
        url.set_query(Some(serializer.finish().as_ref()).filter(|p: &&str| !p.is_empty()));
        Ok(())
    }

    /// Render only our own path component.
    fn render_self_into<'a>(
        &'a self,
        path: &mut Vec<Cow<'a, str>>,
        params: &mut Vec<(Cow<'a, str>, Cow<'a, str>)>,
    );

    /// Render the full path downwards.
    fn render_path_into<'a>(
        &'a self,
        path: &mut Vec<Cow<'a, str>>,
        params: &mut Vec<(Cow<'a, str>, Cow<'a, str>)>,
    );

    /// Parse the target from the provided (segmented) path.
    ///
    /// The path will be the local path, with the prefix already removed.
    fn parse_path(path: &[&str], query_params: &[(Cow<str>, Cow<str>)]) -> Option<Self>;

    fn parse_url(base: impl TryInto<Url>, path: impl TryInto<Url>) -> Option<Self> {
        let base_url = base.try_into().ok()?;
        let full_url = path.try_into().ok()?;
        let base_count = base_url.path_segments().iter().count();
        let path_segments = full_url.path_segments();
        let internal_path = path_segments
            .into_iter()
            .flatten()
            .skip(base_count)
            .collect::<Box<[_]>>();
        let pairs = full_url.query_pairs().into_iter().collect::<Box<[_]>>();
        Self::parse_path(&internal_path, &pairs)
    }
}

/// Maps a `P`arent target onto a `C`hild target and vice versa.
#[derive(Debug, PartialEq)]
pub struct Mapper<P, C> {
    /// Obtain the child target from the parent
    pub downwards: Callback<P, Option<C>>,
    /// Obtain the parent target from the child
    pub upwards: Callback<C, P>,
}

impl<P, C> Clone for Mapper<P, C>
where
    P: Target,
    C: Target,
{
    fn clone(&self) -> Self {
        Self {
            downwards: self.downwards.clone(),
            upwards: self.upwards.clone(),
        }
    }
}

impl<P, C> Mapper<P, C>
where
    P: Target,
    C: Target,
{
    pub fn new<PF, CF>(downwards: PF, upwards: CF) -> Self
    where
        PF: Fn(P) -> Option<C> + 'static,
        CF: Fn(C) -> P + 'static,
    {
        Self {
            downwards: downwards.into(),
            upwards: upwards.into(),
        }
    }

    pub fn new_callback<PF, CF>(downwards: PF, upwards: CF) -> Callback<(), Self>
    where
        PF: Fn(P) -> Option<C> + 'static,
        CF: Fn(C) -> P + 'static,
    {
        Self::new(downwards, upwards).into()
    }
}

impl<P, C> From<Mapper<P, C>> for Callback<(), Mapper<P, C>>
where
    P: Target,
    C: Target,
{
    fn from(mapper: Mapper<P, C>) -> Self {
        Callback::from(move |()| mapper.clone())
    }
}

impl<P, C, PF, CF> From<(PF, CF)> for Mapper<P, C>
where
    P: Target,
    C: Target,
    PF: Fn(P) -> Option<C> + 'static,
    CF: Fn(C) -> P + 'static,
{
    fn from((down, up): (PF, CF)) -> Self {
        Self::new(down, up)
    }
}

pub mod parameter_value {
    use std::{borrow::Cow, str::FromStr};
    pub trait ParameterValues: Sized {
        fn extract_from_params(params: &[(Cow<str>, Cow<str>)], name: &str) -> Option<Self> {
            Self::from_parameter_values(
                params
                    .iter()
                    .filter(|(k, _)| k.as_ref() == name)
                    .map(|(_, v)| v.as_ref()),
            )
        }
        fn from_parameter_values<'a, I: Iterator<Item = &'a str>>(iterator: I) -> Option<Self>;
        fn to_parameter_values(&self) -> Box<[Cow<str>]>;
    }

    pub trait ParameterValue: Sized {
        fn extract_from_params(params: &[(Cow<str>, Cow<str>)], name: &str) -> Option<Self> {
            params
                .iter()
                .filter(|(k, _)| k.as_ref() == name)
                .filter_map(|(_, v)| Self::from_parameter_value(v.as_ref()))
                .next()
        }
        fn from_parameter_value(value: &str) -> Option<Self>;
        fn to_parameter_value(&self) -> Cow<str>;
    }
    impl<V> ParameterValue for V
    where
        V: ToString + FromStr,
    {
        fn from_parameter_value<'a>(value: &str) -> Option<Self> {
            V::from_str(value).ok()
        }

        fn to_parameter_value(&self) -> Cow<str> {
            self.to_string().into()
        }
    }
    impl<V: ParameterValue> ParameterValues for Box<[V]> {
        fn from_parameter_values<'a, I: Iterator<Item = &'a str>>(iterator: I) -> Option<Self> {
            Some(
                iterator
                    .filter_map(|v| V::from_parameter_value(v))
                    .collect(),
            )
            .filter(|values: &Box<[V]>| values.is_empty())
        }

        fn to_parameter_values(&self) -> Box<[Cow<str>]> {
            self.iter().map(|v| v.to_parameter_value().into()).collect()
        }
    }
    impl<V: ParameterValue> ParameterValues for Option<V> {
        fn from_parameter_values<'a, I: Iterator<Item = &'a str>>(iterator: I) -> Option<Self> {
            Some(iterator.filter_map(|v| V::from_parameter_value(v)).next())
        }

        fn to_parameter_values(&self) -> Box<[Cow<str>]> {
            self.iter().map(|v| v.to_parameter_value().into()).collect()
        }
    }
    /*
    impl ParameterValue for String {
        fn from_parameter_values<'a, I: Iterator<Item = &'a str>>(iterator: I) -> Option<Self> {
            iterator.map(|v| v.to_string()).next()
        }

        fn to_parameter_values(&self) -> Box<[Cow<str>]> {
            Box::new([self.as_str().into()])
        }
    }

    impl ParameterValue for Box<str> {
        fn from_parameter_values<'a, I: Iterator<Item = &'a str>>(iterator: I) -> Option<Self> {
            iterator.map(|v| v.into()).next()
        }

        fn to_parameter_values(&self) -> Box<[Cow<str>]> {
            Box::new([self.as_ref().into()])
        }
    }*/
}
