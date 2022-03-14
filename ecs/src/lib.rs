mod component_registry;

pub use component_registry::ComponentRegistry;

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn component_registry() {
        struct A(u8);
        struct B(&'static str);
        struct C(u16);

        let mut reg = ComponentRegistry::default();
        let a_id = reg.register::<A>();
        let b_id = reg.register::<B>();

        assert_eq!(size_of::<A>(), reg[a_id].layout().size());
        assert_eq!(size_of::<B>(), reg[b_id].layout().size());

        assert_eq!(Some(&reg[a_id]), reg.info::<A>());
        assert_eq!(Some(&reg[b_id]), reg.info::<B>());
        assert_eq!(None, reg.info::<C>());

        assert_eq!(Some(a_id), reg.id::<A>());
        assert_eq!(Some(b_id), reg.id::<B>());
        assert_eq!(None, reg.id::<C>());
    }
}
