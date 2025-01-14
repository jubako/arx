use crate::BaseError;

use super::common::*;
use super::Arx;
use jbk::reader::Range;

pub trait Operator<Context, Builder: FullBuilderTrait> {
    type Error;
    fn on_start(&self, context: &mut Context) -> Result<(), Self::Error>;
    fn on_stop(&self, context: &mut Context) -> Result<(), Self::Error>;
    fn on_directory_enter(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Dir,
    ) -> Result<bool, Self::Error>;
    fn on_directory_exit(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Dir,
    ) -> Result<(), Self::Error>;
    fn on_file(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::File,
    ) -> Result<(), Self::Error>;
    fn on_link(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Link,
    ) -> Result<(), Self::Error>;
}

pub struct Walker<'a, Context> {
    arx: &'a Arx,
    context: Context,
}

impl<'a, Context> Walker<'a, Context> {
    pub fn new(arx: &'a Arx, context: Context) -> Self {
        Self { arx, context }
    }

    pub fn run<B, O>(&mut self, op: &O) -> Result<(), O::Error>
    where
        B: FullBuilderTrait,
        O: Operator<Context, B>,
        O::Error: From<BaseError>,
    {
        let builder = RealBuilder::<B>::new(&self.arx.properties);

        op.on_start(&mut self.context)?;
        self._run(&self.arx.root_index, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    pub fn run_from_range<R: Range, B, O>(&mut self, op: &O, range: &R) -> Result<(), O::Error>
    where
        B: FullBuilderTrait,
        O: Operator<Context, B>,
        O::Error: From<BaseError>,
    {
        let builder = RealBuilder::<B>::new(&self.arx.properties);

        op.on_start(&mut self.context)?;
        self._run(range, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    fn _run<R: Range, B, O>(
        &mut self,
        range: &R,
        builder: &RealBuilder<B>,
        op: &O,
    ) -> Result<(), O::Error>
    where
        B: FullBuilderTrait,
        O: Operator<Context, B>,
        O::Error: From<BaseError>,
    {
        let read_entry = ReadEntry::new(range, builder);
        for entry in read_entry {
            match entry? {
                Entry::File(e) => op.on_file(&mut self.context, &e)?,
                Entry::Link(e) => op.on_link(&mut self.context, &e)?,
                Entry::Dir(range, e) => {
                    if op.on_directory_enter(&mut self.context, &e)? {
                        self._run(&range, builder, op)?;
                    }
                    op.on_directory_exit(&mut self.context, &e)?;
                }
            }
        }
        Ok(())
    }
}
