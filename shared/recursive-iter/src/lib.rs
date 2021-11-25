pub struct RecursiveIter<F: Frame> {
    context: F::Context,
    stack: Vec<F>,
}

pub struct Yield<T>(pub T);

impl<T> Yield<T> {
    pub fn with_return<F: Frame<Item = T>>(self, return_: bool) -> EvalResult<F> {
        EvalResult {
            yield_: Some(self.0),
            call: None,
            return_,
        }
    }
}

pub struct Call<F: Frame>(pub F);

impl<F: Frame> Call<F> {
    pub fn with_return(self, return_: bool) -> EvalResult<F> {
        EvalResult {
            yield_: None,
            call: Some(self.0),
            return_,
        }
    }
}

pub struct EvalResult<F: Frame> {
    yield_: Option<F::Item>,
    call: Option<F>,
    return_: bool,
}

impl<F: Frame> EvalResult<F> {
    pub fn yield_(item: F::Item) -> Self {
        Self {
            yield_: Some(item),
            call: None,
            return_: false,
        }
    }

    pub fn recursive_call(frame: F) -> Self {
        Self {
            yield_: None,
            call: Some(frame),
            return_: false,
        }
    }

    pub fn return_() -> Self {
        Self {
            yield_: None,
            call: None,
            return_: true,
        }
    }
}

impl<F: Frame> From<Yield<F::Item>> for EvalResult<F> {
    fn from(yield_: Yield<F::Item>) -> Self {
        EvalResult::yield_(yield_.0)
    }
}

impl<F: Frame> From<Call<F>> for EvalResult<F> {
    fn from(recurse: Call<F>) -> Self {
        EvalResult::recursive_call(recurse.0)
    }
}

pub trait Frame: Sized {
    type Item;
    type Context;

    fn eval(&mut self, context: &mut Self::Context) -> EvalResult<Self>;
}

impl<F: Frame> RecursiveIter<F> {
    pub fn new(context: F::Context, initial_frame: F) -> Self {
        Self {
            context,
            stack: vec![initial_frame],
        }
    }
}

impl<F: Frame> Iterator for RecursiveIter<F> {
    type Item = F::Item;

    fn next(&mut self) -> Option<F::Item> {
        loop {
            match self.stack.last_mut() {
                Some(frame) => {
                    let result = frame.eval(&mut self.context);
                    if result.return_ {
                        self.stack.pop();
                    }
                    if let Some(frame) = result.call {
                        self.stack.push(frame);
                    }
                    if let Some(item) = result.yield_ {
                        return Some(item);
                    }
                }
                None => return None,
            }
        }
    }
}
