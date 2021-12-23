use std::collections::{HashMap, HashSet};

//this is stupid...
struct TopicLevel {
    subtopics: HashMap<String, TopicLevel>,
    subscribers: HashSet<String>,
}

impl Default for Topics {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicLevel {
    fn new() -> Self {
        TopicLevel {
            subtopics: HashMap::new(),
            subscribers: HashSet::new(),
        }
    }
}

pub struct Topics {
    topics: HashMap<String, TopicLevel>,
}

impl Topics {
    pub fn new() -> Self {
        Topics {
            topics: HashMap::new(),
        }
    }

    fn resolve_path_mut(&mut self, path: &str) -> &mut TopicLevel {
        let mut sections = path.split('/');
        let starting_point = match path.chars().next().unwrap() {
            '/' => "/",
            _ => sections.next().unwrap(),
        };
        if !self.topics.contains_key(starting_point) {
            self.topics
                .insert(starting_point.to_owned(), TopicLevel::new());
        }
        let mut target = self.topics.get_mut(starting_point).unwrap();

        for section in sections {
            if !target.subtopics.contains_key(section) {
                target
                    .subtopics
                    .insert(section.to_owned(), TopicLevel::new());
            }
            target = target.subtopics.get_mut(section).unwrap();
        }
        target
    }
    fn resolve_path(&self, path: &str) -> Option<&TopicLevel> {
        let mut sections = path.split('/');
        let starting_point = match path.chars().next().unwrap() {
            '/' => "/",
            _ => sections.next().unwrap(),
        };
        if !self.topics.contains_key(starting_point) {
            return None;
        }
        let mut target = self.topics.get(starting_point).unwrap();

        for section in sections {
            if !target.subtopics.contains_key(section) {
                return None;
            }
            target = target.subtopics.get(section).unwrap();
        }
        Some(target)
    }

    pub fn subscribe(&mut self, path: &str, client: &str) {
        if path.is_empty() {
            return;
        }
        let target = self.resolve_path_mut(path);
        target.subscribers.insert(client.to_owned());
    }
    pub fn unsbscribe(&mut self, path: &str, client: &str) {
        if path.is_empty() {
            return;
        }
        let target = self.resolve_path_mut(path);
        target.subscribers.remove(client);
    }
    pub fn get_subscribed(&self, path: &str) -> Option<&HashSet<String>> {
        if path.is_empty() {
            return None;
        }
        self.resolve_path(path).map(|t| &t.subscribers)
    }
}
