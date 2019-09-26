custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(AssociativityVariants), IterVariantNames(AssociativityVariantNames))]
    /// When tabulating the vote for a bunch of boolean indicators,
    /// this determines how many true values are required for the vote to pass. 
    pub enum VotingRule {
        /// All values must be false
        None,
        /// Exactly one is true
        One,
        /// One or more are true
        Any,
        /// Less than half (but at least one) are true
        Minority,
        /// Half or more are true
        Half,
        /// More than half are true
        Majority,
        /// Two-thirds or more are true
        TwoThirds,
        /// Exactly one is false
        AllButOne,
        /// All are true
        All,
        /// All are true OR all are false
        Unanimous
    }
}
