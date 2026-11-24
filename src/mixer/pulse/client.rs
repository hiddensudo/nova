use std::{
    cell::RefCell,
    rc::Rc,
    thread,
    time::{Duration, Instant},
};

use libpulse_binding::{
    context::{Context, FlagSet, State},
    mainloop::standard::{IterateResult, Mainloop},
    operation::{Operation, State as OperationState},
};

thread_local! {
    static CONNECTION: RefCell<Option<Client>> = const { RefCell::new(None) };
}

pub(super) fn with_client<T>(
    task: impl FnOnce(&mut Client) -> Result<T, String>,
) -> Result<T, String> {
    CONNECTION.with_borrow_mut(|slot| {
        if let Some(client) = slot
            && (client.pump().is_err() || client.context.get_state() != State::Ready)
        {
            *slot = None;
        }
        if slot.is_none() {
            *slot = Some(Client::connect()?);
        }
        let result = task(slot.as_mut().expect("connected audio client"));
        if result.is_err() {
            *slot = None;
        }
        result
    })
}

pub(super) struct Client {
    // The context must be released before its main loop.
    pub context: Context,
    mainloop: Mainloop,
}

impl Client {
    fn connect() -> Result<Self, String> {
        let mainloop = Mainloop::new().ok_or("Could not create the audio main loop.")?;
        let mut context =
            Context::new(&mainloop, "Nova Control").ok_or("Could not create the audio client.")?;
        context
            .connect(None, FlagSet::NOAUTOSPAWN, None)
            .map_err(|e| format!("Audio server: {e}"))?;
        let mut client = Self { context, mainloop };
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match client.context.get_state() {
                State::Ready => return Ok(client),
                State::Failed | State::Terminated => return Err(client.error("connect")),
                _ => client.step(deadline)?,
            }
        }
    }

    fn error(&self, action: &str) -> String {
        format!("Audio server ({action}): {}", self.context.errno())
    }

    fn pump(&mut self) -> Result<(), String> {
        match self.mainloop.iterate(false) {
            IterateResult::Success(_) => Ok(()),
            _ => Err(self.error("main loop")),
        }
    }

    fn step(&mut self, deadline: Instant) -> Result<(), String> {
        if Instant::now() >= deadline {
            return Err("The audio server did not respond in time.".into());
        }
        self.pump()?;
        thread::sleep(Duration::from_millis(1));
        Ok(())
    }

    pub fn wait<F: ?Sized>(&mut self, mut operation: Operation<F>) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while operation.get_state() == OperationState::Running {
            if let Err(error) = self.step(deadline) {
                operation.cancel();
                return Err(error);
            }
            if self.context.get_state() != State::Ready {
                operation.cancel();
                return Err(self.error("disconnected"));
            }
        }
        if operation.get_state() == OperationState::Done {
            Ok(())
        } else {
            Err(self.error("request cancelled"))
        }
    }

    pub fn request<T: 'static, F: ?Sized>(
        &mut self,
        name: &str,
        run: impl FnOnce(&mut Context, Rc<RefCell<Option<T>>>) -> Operation<F>,
    ) -> Result<T, String> {
        let result = Rc::new(RefCell::new(None));
        let operation = run(&mut self.context, result.clone());
        self.wait(operation)?;
        result.take().ok_or_else(|| self.error(name))
    }

    pub fn success(
        &mut self,
        name: &str,
        run: impl FnOnce(&mut Context, Box<dyn FnMut(bool)>) -> Operation<dyn FnMut(bool)>,
    ) -> Result<(), String> {
        if self.request(name, |context, result| {
            run(context, Box::new(move |ok| *result.borrow_mut() = Some(ok)))
        })? {
            Ok(())
        } else {
            Err(self.error(name))
        }
    }
}
