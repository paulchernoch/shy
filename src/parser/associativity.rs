
//..................................................................

/// Operator Associativity
custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(AssociativityVariants), IterVariantNames(AssociativityVariantNames))]
    pub enum Associativity {
        Left,
        Right,
        None
    }
}
