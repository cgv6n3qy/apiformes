use apiformes_packet::prelude::*;
use async_recursion::async_recursion;
use bitflags::bitflags;
use futures::future::BoxFuture;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::trace;
type ClientId = Arc<str>;
type SubTopic = Arc<str>;

bitflags! {
    pub struct SubscriptionFlags: u8 {
        const NO_LOCAL              = 0b0000_0001;
        const RETAIN_AS_PUBLISHED   = 0b0000_0010;
    }
}

#[derive(Clone)]
pub struct SubscriptionInfo {
    pub qos: QoS,
    pub flags: SubscriptionFlags,
}

impl SubscriptionInfo {
    fn new(qos: QoS, flags: SubscriptionFlags) -> Self {
        SubscriptionInfo { qos, flags }
    }
}

/// This Block structure is designed to have minimal concurrency overhead,
/// Justification for subscribers Lock:
/// Lets assume that client is publishing to `/hello/world`, that client would
/// acquire read lock for `/`  then `/hello` and then aquire another read lock
/// for `/hello/world` and a one more read-lock for the subscribers.
/// Now Lets assume that concurrently, another client is subscribing to `/hello` it
/// would still aquire read lock to `/` and `/hello`, but a write lock to the
/// subscribers members inside `/hello`. This way both operations could be
/// completed concurrently, but at the cost of extra allocations for the fine grained
/// locking.
///
/// There are two areas for improvements here:
/// a) use concurrent hashmap instead of the normal hashmap, There is chashmap-async
/// but it is not clear if it is reliable
/// b) use slab allocator for blocks, When traversing blocks the process would be
/// cache friendly, there is crate slab which is made by tokio folks
struct BlockInner {
    hash_wildcard: RwLock<HashMap<ClientId, SubscriptionInfo>>,
    subscribers: RwLock<HashMap<ClientId, SubscriptionInfo>>,
    // the `+` wild card is stored in here
    // TODO lazy static the `+` subtopic
    sub_blocks: HashMap<SubTopic, Block>,
}

impl Default for BlockInner {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockInner {
    fn new() -> Self {
        BlockInner {
            hash_wildcard: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            sub_blocks: HashMap::new(),
        }
    }
    async fn insert_into_hash(&self, clientid: Arc<str>, qos: QoS, flags: SubscriptionFlags) {
        trace!("Inserting {} into hash", clientid);
        let info = SubscriptionInfo::new(qos, flags);
        self.hash_wildcard.write().await.insert(clientid, info);
    }
    async fn remove_from_hash(&self, clientid: Arc<str>) {
        self.hash_wildcard.write().await.remove(&clientid);
    }
    async fn insert_into_subs(&self, clientid: Arc<str>, qos: QoS, flags: SubscriptionFlags) {
        trace!("Inserting {} into subs", clientid);
        let info = SubscriptionInfo::new(qos, flags);
        self.subscribers.write().await.insert(clientid, info);
    }
    async fn remove_from_subs(&self, clientid: Arc<str>) {
        self.subscribers.write().await.remove(&clientid);
    }
    async fn collect_hash_wildcard(&self, subs: &mut HashMap<ClientId, SubscriptionInfo>) {
        for (clientid, info) in self.hash_wildcard.read().await.iter() {
            match subs.entry(clientid.clone()) {
                Entry::Vacant(e) => {
                    e.insert(info.clone());
                }
                Entry::Occupied(mut e) => {
                    if e.get().qos < info.qos {
                        e.insert(info.clone());
                    }
                }
            }
        }
    }

    async fn collect_subscribers(&self, subs: &mut HashMap<ClientId, SubscriptionInfo>) {
        trace!(
            "Collecting subsscribers total = {}",
            self.subscribers.read().await.keys().count()
        );
        for (clientid, info) in self.subscribers.read().await.iter() {
            match subs.entry(clientid.clone()) {
                Entry::Vacant(e) => {
                    e.insert(info.clone());
                }
                Entry::Occupied(mut e) => {
                    if e.get().qos < info.qos {
                        e.insert(info.clone());
                    }
                }
            }
        }
    }
}

struct Block {
    inner: RwLock<BlockInner>,
}

impl Block {
    fn new() -> Self {
        Block {
            inner: RwLock::new(BlockInner::new()),
        }
    }

    #[inline(always)]
    async fn read(&self) -> RwLockReadGuard<'_, BlockInner> {
        self.inner.read().await
    }

    #[inline(always)]
    async fn write(&self) -> RwLockWriteGuard<'_, BlockInner> {
        self.inner.write().await
    }

    #[async_recursion]
    async fn visit<'a, S>(
        &self,
        mut sections: S,
        create: bool,
        run: impl FnOnce(&Block, bool) -> BoxFuture<()> + Send + 'async_recursion,
    ) where
        S: Iterator<Item = &'a str> + Send,
    {
        let x = sections.next();
        trace!("Visiting {:?}", x);
        match x {
            Some("#") => run(self, true).await,
            None => run(self, false).await,
            Some(section) => {
                if create {
                    self.create_if_not_existing(section).await;
                }
                let raii = self.read().await;
                if let Some(sub_block) = raii.sub_blocks.get(section) {
                    sub_block.visit(sections, create, run).await;
                }
            }
        }
    }

    #[async_recursion]
    async fn collect_subs<'a, S>(
        &self,
        subs: &mut HashMap<ClientId, SubscriptionInfo>,
        mut sections: S,
    ) where
        S: Iterator<Item = &'a str> + Send + Sync + Clone,
    {
        let raii = self.read().await;
        raii.collect_hash_wildcard(subs).await;
        if let Some(section) = sections.next() {
            // somewhere in the middle
            trace!("in collect_subs, section = `{}`", section);
            if let Some(sub_block) = raii.sub_blocks.get("+") {
                sub_block.collect_subs(subs, sections.clone()).await;
            }
            if let Some(sub_block) = raii.sub_blocks.get(section) {
                sub_block.collect_subs(subs, sections).await;
            }
        } else {
            // reached the end
            raii.collect_subscribers(subs).await;
        }
    }

    async fn contains_subtopic(&self, subtopic: &str) -> bool {
        self.read().await.sub_blocks.contains_key(subtopic)
    }
    async fn create_if_not_existing(&self, subtopic: &str) {
        if !self.contains_subtopic(subtopic).await {
            let block = Block::new();
            self.write()
                .await
                .sub_blocks
                .insert(Arc::from(subtopic), block);
        }
    }
}

/// There is important invariant to maintain here, the one and only use of
/// `reverse_index` is when client disconnects and we want to remove all his
/// subscriptions, we need to guarantee that for all entries in `reverse_index`
/// there must always be equivalent entry in `topics`. As such, when we insert
/// we insert into `topics` first but removal is done in reverse order
pub struct TopicsTable {
    root_block: Block,
    reverse_index: RwLock<HashMap<ClientId, HashSet<SubTopic>>>,
    //TODO we can have slab allocator for Block here
    // and another slab allocator for subscription info
    // but first to make sure that this is not just
    // useless complexity, one needs to establish
    // notion of performance metric regarding cache hits/
    // cache misses and memory fragmentation which requires
    // more complex benchmarks
}

impl Default for TopicsTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicsTable {
    pub fn new() -> Self {
        TopicsTable {
            root_block: Block::new(),
            reverse_index: RwLock::new(HashMap::new()),
        }
    }
    fn topic_to_subtopics<'a>(&self, topic: &'a str) -> impl Iterator<Item = &'a str> + Clone {
        let mut sections = topic.split('/');
        let starting_point = match sections.next().unwrap() {
            "" => "/",
            x => x,
        };
        std::iter::once(starting_point).chain(sections)
    }
    /// `topic`: the topic you want to visit
    /// `create`: create all missing nodes if they does not exist
    /// the first argument of f is the subtopic block to be visited, and the
    /// second argument is bool that is true if we are accessing hash wildcard inside that topic
    /// Note: this visit functions assumes that topic is valid MQTT protocol topic as per the specs
    /// TODO: keep watching for https://github.com/rust-lang/rust/issues/62290
    /// This way you may have async futures, thus avoid returning a boxed value everytime.
    async fn visit(
        &self,
        topic: &str,
        create: bool,
        run: impl FnOnce(&Block, bool) -> BoxFuture<()> + Send,
    ) {
        let sections = self.topic_to_subtopics(topic);
        self.root_block.visit(sections, create, run).await;
    }

    // the reason we made clientid Arc but topic reference is that topic will be sliced anyways so
    // no need to do expensive AtomicUsize increment
    async fn topics_add(
        &self,
        clientid: Arc<str>,
        topic: &str,
        qos: QoS,
        flags: SubscriptionFlags,
    ) {
        self.visit(topic, true, |block: &Block, is_hash: bool| {
            Box::pin(async move {
                if is_hash {
                    block
                        .read()
                        .await
                        .insert_into_hash(clientid, qos, flags)
                        .await;
                } else {
                    block
                        .read()
                        .await
                        .insert_into_subs(clientid, qos, flags)
                        .await;
                }
            })
        })
        .await
    }
    async fn reverse_index_add(&self, clientid: Arc<str>, topic: Arc<str>) {
        match self.reverse_index.write().await.entry(clientid) {
            Entry::Occupied(mut e) => {
                e.get_mut().insert(topic);
            }
            Entry::Vacant(e) => {
                let s = e.insert(HashSet::new());
                s.insert(topic);
            }
        }
    }
    pub async fn subscribe(
        &self,
        clientid: Arc<str>,
        topic: Arc<str>,
        qos: QoS,
        flags: SubscriptionFlags,
    ) {
        self.topics_add(clientid.clone(), &topic, qos, flags).await;
        self.reverse_index_add(clientid, topic).await;
    }
    async fn reverse_index_remove(&self, clientid: &str, topic: &str) {
        let mut raii = self.reverse_index.write().await;
        let mut cleanup = false;
        if let Some(set) = raii.get_mut(clientid) {
            set.remove(topic);
            cleanup = set.is_empty();
        }
        if cleanup {
            raii.remove(clientid);
        }
    }
    //TODO replace Arc<str> with &str
    async fn topic_remove(&self, clientid: Arc<str>, topic: &str) {
        self.visit(topic, false, |block: &Block, is_hash: bool| {
            Box::pin(async move {
                if is_hash {
                    block.read().await.remove_from_hash(clientid).await
                } else {
                    block.read().await.remove_from_subs(clientid).await
                }
            })
        })
        .await
    }
    pub async fn unsubscribe(&self, clientid: Arc<str>, topic: &str) {
        self.reverse_index_remove(&clientid, topic).await;
        self.topic_remove(clientid, topic).await;
    }
    pub async fn reverse_index_remove_all(&self, clientid: &str) -> Option<HashSet<SubTopic>> {
        self.reverse_index.write().await.remove(clientid)
    }
    pub async fn unsubscribe_all(&self, clientid: Arc<str>) {
        if let Some(topics) = self.reverse_index_remove_all(&clientid).await {
            for topic in topics {
                self.topic_remove(clientid.clone(), &topic).await
            }
        }
    }
    pub async fn get_all_subscribed(&self, topic: &str) -> HashMap<ClientId, SubscriptionInfo> {
        let mut subs = HashMap::new();
        let sections = self.topic_to_subtopics(topic);
        trace!(
            "collected_sections {:?}",
            sections.clone().collect::<Vec<_>>()
        );
        self.root_block.collect_subs(&mut subs, sections).await;
        subs
    }
}
