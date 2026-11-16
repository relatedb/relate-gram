CREATE (:Person {`@id`: 'ada', name: 'Ada', born: 1815, note: 'kept'}), (:Person {`@id`: 'grace', name: 'Grace'});
MATCH (a {`@id`: 'ada'}), (g {`@id`: 'grace'}) CREATE (a)-[:KNOWS {`@id`: 'k', since: 1840}]->(g);
