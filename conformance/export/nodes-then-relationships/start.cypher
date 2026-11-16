CREATE (a:Person {name: 'Ada', born: 1815}), (b:Person {name: 'Grace'}), (a)-[:KNOWS {since: 1843}]->(b);
