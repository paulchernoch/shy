
//..................................................................

custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(AssociativityVariants), IterVariantNames(AssociativityVariantNames))]
    /// Operator Associativity
    pub enum Associativity {
        Left,
        Right,
        None
    }
}
