CREATE (:P {`@id`: 'x'}), (:P {`@id`: 'y'});
MATCH (a:P {`@id`: 'x'}), (b:P {`@id`: 'y'}) CREATE (a)-[:R {`@id`: 'x'}]->(b);
