pub type Index = u64;
pub type Term  = u64;

#[tonic::async_trait]
pub trait Tracker: Sync + Send + 'static {
    type Entity;
    
    async fn propagate(&self, entity: &Self::Entity) -> crate::Result<Self::Entity>;

    fn get_last_log_index(&self) -> Index;

    fn get_last_log_term(&self) -> Term;

    fn get_last_commited_index(&self) -> Index;

    fn get_log_entity(&self, index: Index) -> &Self::Entity;

    fn get_log_term(&self, index: Index) -> Term;

    fn get_last_snapshot_index(&self) -> Index;

    fn get_last_snapshot_term(&self) -> Term;

    fn get_snapshot_no(&self) -> u64;

    fn append_log(&mut self, entity: Self::Entity, term: Term) -> crate::Result<Index>;

    fn delete_last_log(&mut self) -> crate::Result<()>;

    async fn take_snapshot(&mut self) -> crate::Result<()>;

    async fn load_snappshot(&mut self, entity: &Self::Entity, len: u64, last_log_term: Term) -> crate::Result<()>;

    async fn read_snapshot(&self) -> crate::Result<String>;

    async fn commit(&mut self, index: Index) -> crate::Result<Self::Entity>;
}
