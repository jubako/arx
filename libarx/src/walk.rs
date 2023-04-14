use super::common::*;
use jbk::reader::Range;
use jubako as jbk;

pub trait Operator<Context, E: EntryDef> {
    fn on_start(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_stop(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_directory_enter(&self, context: &mut Context, entry: &E::Dir) -> jbk::Result<()>;
    fn on_directory_exit(&self, context: &mut Context, entry: &E::Dir) -> jbk::Result<()>;
    fn on_file(&self, context: &mut Context, entry: &E::File) -> jbk::Result<()>;
    fn on_link(&self, context: &mut Context, entry: &E::Link) -> jbk::Result<()>;
}

pub struct Walker<'a, Context> {
    arx: &'a Arx,
    context: Context,
}

impl<'a, Context> Walker<'a, Context> {
    pub fn new(arx: &'a Arx, context: Context) -> Self {
        Self { arx, context }
    }

    pub fn run<B>(
        &mut self,
        index: jbk::reader::Index,
        op: &dyn Operator<Context, B::Entry>,
    ) -> jbk::Result<()>
    where
        B: FullBuilder,
    {
        let properties = self.arx.create_properties(&index)?;
        let builder = RealBuilder::<B>::new(&properties);

        op.on_start(&mut self.context)?;
        self._run(&index, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    fn _run<R: Range, B>(
        &mut self,
        range: &R,
        builder: &RealBuilder<B>,
        op: &dyn Operator<Context, B::Entry>,
    ) -> jbk::Result<()>
    where
        B: FullBuilder,
    {
        let read_entry = ReadEntry::new(range, builder);
        for entry in read_entry {
            match entry? {
                Entry::File(e) => op.on_file(&mut self.context, &e)?,
                Entry::Link(e) => op.on_link(&mut self.context, &e)?,
                Entry::Dir(range, e) => {
                    op.on_directory_enter(&mut self.context, &e)?;
                    self._run(&range, builder, op)?;
                    op.on_directory_exit(&mut self.context, &e)?;
                }
            }
        }
        Ok(())
    }
}
