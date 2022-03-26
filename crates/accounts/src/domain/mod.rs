pub mod account;
pub mod events;
pub mod money;

// A mix of the Result (Either) with Writer monad
// https://adit.io/posts/2013-06-10-three-useful-monads.html
#[derive(Clone, Debug)]
pub enum DomainResult<T, TErr, TEvent>
where
    T: Clone + std::fmt::Debug,
    TErr: Clone + std::fmt::Debug,
    TEvent: Clone + std::fmt::Debug,
{
    Ok { data: T, events: Vec<TEvent> },
    Err(TErr),
}

impl<T, TErr, TEvent> DomainResult<T, TErr, TEvent>
where
    T: Clone + std::fmt::Debug,
    TErr: Clone + std::fmt::Debug,
    TEvent: Clone + std::fmt::Debug,
{
    pub fn map<TResult, FOk, FErr>(self, fok: FOk, ferr: FErr) -> TResult
    where
        FOk: Fn(T, Vec<TEvent>) -> TResult,
        FErr: Fn(TErr) -> TResult,
    {
        match self {
            DomainResult::Ok { data, events } => fok(data, events),
            DomainResult::Err(err) => ferr(err),
        }
    }

    pub fn unwrap(self) -> (T, Vec<TEvent>) {
        match self {
            DomainResult::Ok { data, events } => (data, events),
            DomainResult::Err(err) => panic!("{:?}", err),
        }
    }

    pub fn unwrap_data(self) -> T {
        match self {
            DomainResult::Ok { data, .. } => data,
            DomainResult::Err(err) => panic!("{:?}", err),
        }
    }

    pub fn unwrap_events(self) -> Vec<TEvent> {
        match self {
            DomainResult::Ok { events, .. } => events,
            DomainResult::Err(err) => panic!("{:?}", err),
        }
    }

    pub fn is_err(self) -> bool {
        match self {
            DomainResult::Ok { .. } => false,
            DomainResult::Err(_) => true,
        }
    }

    pub fn expect_err(self, msg: impl AsRef<str>) -> DomainResult<T, TErr, TEvent> {
        match self {
            DomainResult::Err(err) => DomainResult::Err(err),
            _ => panic!("{}", msg.as_ref()),
        }
    }
}
