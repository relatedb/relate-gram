CREATE (:P {email: 'a'}), (:P {email: 'b'});
MATCH (a:P {email: 'a'}), (b:P {email: 'b'}) CREATE (a)-[:R]->(b);
