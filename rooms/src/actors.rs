use std::{
    rc::Rc,
    cell::RefCell,
};

pub struct Addr<A: Actor>(Rc<RefCell<A>>, Rc<RefCell<A::Context>>);

impl<A: Actor> Addr<A> {
    pub fn send<M>(&self, msg: M) -> A::Response
        where A: Handle<M>, M: Message,
    {
        let mut ctx = self.1.borrow_mut();
        self.0.borrow_mut().handle(msg, &mut *ctx)
    }
}

pub trait Actor: Sized + 'static {
    type Context: ActorContext;
}

pub trait ActorContext {}

pub trait Message {
    type Response: 'static;
}

pub trait Handle<M>
    where Self: Actor, M: Message
{
    type Response: Response<Self, M>;
    fn handle(&mut self, msg: M, ctx: &mut Self::Context) -> Self::Response;
}

pub trait Response<A: Actor, M: Message> {
    fn handle<R>(self, ctx: &mut A::Context, tx: Option<R>);
}

impl<A: Actor, M: Message> Response<A, M> for () {
    fn handle<R>(self, _ctx: &mut A::Context, _tx: Option<R>) {}
}

impl<A: Actor, M: Message> Response<A, M> for String {
    fn handle<R>(self, _ctx: &mut A::Context, _tx: Option<R>) {}
}


pub struct Sys<C: ActorContext> {
    ctx: Rc<RefCell<C>>,
}

impl<C: ActorContext> Sys<C> {
    pub fn new(ctx: C) -> Self {
        Self {
            ctx: Rc::new(RefCell::new(ctx)),
        }
    }
    pub fn spawn<A>(&mut self, actor: A) -> Addr<A>
        where A: Actor<Context=C>
    {
        let actor = Rc::new(RefCell::new(actor));
        Addr(actor, self.ctx.clone())
    }
}

pub macro send($to:ident . $msg:ident ( $($arg:tt)* )) {
    $to.send($msg { $($arg)* })
}

pub macro actor_impl {
    (struct $actor:ident<$ctx_t:ty> { $($data:tt)* }) => {
        struct $actor {
            $($data)*
        }
        impl Actor for $actor {
            type Context = $ctx_t;
        }
    },
    ($vis:vis struct $actor:ident<$ctx_t:ty> { $($data:tt)* }) => {
        $vis struct $actor {
            $($data)*
        }
        impl Actor for $actor {
            type Context = $ctx_t;
        }
    }
}

pub macro message {
    (struct $name:ident { $($content:tt)* }  -> $resp:ty) => {
        struct $name { $($content)* }
        impl Message for $name {
            type Response = $resp;
        }
    },
    ($vis:vis struct $name:ident { $($content:tt)* }  -> $resp:ty) => {
        $vis struct $name { $($content)* }
        impl Message for $name {
            type Response = $resp;
        }
    }
}

pub macro actor_handle {
    (fn $actor:ident :: $name:ident (
        $(&mut $self:ident,)? $ctx:ident, $msg:ident $(,)?
    ) -> $resp:ty {
        $($body:tt)*
    }) => {
        impl Handle<$name> for $actor {
            type Response = $resp;
            fn handle($(&mut $self)?, $msg: $name, $ctx: &mut Self::Context) -> Self::Response {
                $($body)*
            }
        }
    }
}

pub macro actor {
    (fn $actor:ident $name:ident (
        $(&mut $self:ident,)?
        $ctx:ident,
        $msg:ident: { $($content: tt)* }
        $(,)?
    ) -> $resp:ty { $($body:tt)* }) => {
        message! { struct $name { $($content)* } -> $resp }
        actor_handle!(fn $actor :: $name ($(&mut $self,)? $ctx, $msg) -> $resp {
            $($body)*
        });
    },

    (pub fn $actor:ident $name:ident (
        $(&mut $self:ident,)?
        $ctx:ident,
        $msg:ident: { $($content: tt)* }
        $(,)?
    ) -> $resp:ty { $($body:tt)* }) => {
        message! { struct $name { $($content)* } -> $resp }
        actor_handle!(fn $actor :: $name ($(&mut $self,)? $ctx, $msg) -> $resp {
            $($body)*
        });
    },

    (
        struct $actor:ident<$ctx_t:ty> { $($data:tt)* }
        impl $actor_n:ident { $($tok:tt)* }
    ) => {
        actor_impl! { struct $actor<$ctx_t> { $($data)* } }
        actor! { impl $actor_n { $($tok)* } }
    },
    (
        $vis:vis struct $actor:ident<$ctx_t:ty> { $($data:tt)* }
        impl $actor_n:ident { $($tok:tt)* }
    ) => {
        actor_impl! { $vis struct $actor<$ctx_t> { $($data)* } }
        actor! { impl $actor_n { $($tok)* } }
    },

    (impl $actor:ident {
        $(
            fn $name:ident (
                $(&mut $self:ident,)?
                $ctx:ident,
                $msg:ident: { $($content: tt)* }
            ) -> $resp:ty
            {
                $($body:tt)*
            }
        )*
    }) => {
        $(
            message! { struct $name { $($content)* } -> $resp }
            actor_handle!(fn $actor :: $name ($(&mut $self,)? $ctx, $msg) -> $resp {
                $($body)*
            });
        )*
    }
}
