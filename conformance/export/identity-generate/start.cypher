CREATE (:P {`@id`: 'ada'}), (:P {`@id`: 'grace'});
MATCH (a {`@id`: 'ada'}), (b {`@id`: 'grace'}) CREATE (a)-[:KNOWS {`@id`: 'k'}]->(b);
