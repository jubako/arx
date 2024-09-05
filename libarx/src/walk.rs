use super::common::*;
use super::Arx;
use jbk::reader::Range;

pub trait Operator<Context, Builder: FullBuilderTrait> {
    fn on_start(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_stop(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_directory_enter(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Dir,
    ) -> jbk::Result<bool>;
    fn on_directory_exit(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Dir,
    ) -> jbk::Result<()>;
    fn on_file(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::File,
    ) -> jbk::Result<()>;
    fn on_link(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Link,
    ) -> jbk::Result<()>;
}

pub struct Walker<'a, Context> {
    arx: &'a Arx,
    context: Context,
}

impl<'a, Context> Walker<'a, Context> {
    pub fn new(arx: &'a Arx, context: Context) -> Self {
        Self { arx, context }
    }

    pub fn run<B>(&mut self, op: &dyn Operator<Context, B>) -> jbk::Result<()>
    where
        B: FullBuilderTrait,
    {
        let builder = RealBuilder::<B>::new(&self.arx.properties);

        op.on_start(&mut self.context)?;
        self._run(&self.arx.root_index, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    pub fn run_from_range<R: Range, B>(
        &mut self,
        op: &dyn Operator<Context, B>,
        range: &R,
    ) -> jbk::Result<()>
    where
        B: FullBuilderTrait,
    {
        let builder = RealBuilder::<B>::new(&self.arx.properties);

        op.on_start(&mut self.context)?;
        self._run(range, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    fn _run<R: Range, B>(
        &mut self,
        range: &R,
        builder: &RealBuilder<B>,
        op: &dyn Operator<Context, B>,
    ) -> jbk::Result<()>
    where
        B: FullBuilderTrait,
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
