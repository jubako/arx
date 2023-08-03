pub trait EntryDef {
    type File;
    type Link;
    type Dir;
}

impl<F, L, D> EntryDef for (F, L, D) {
    type File = F;
    type Link = L;
    type Dir = D;
}

pub enum Entry<E: EntryDef> {
    File(E::File),
    Link(E::Link),
    Dir(jbk::EntryRange, E::Dir),
}
